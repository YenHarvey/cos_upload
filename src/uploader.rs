use crate::config::Config;
use crate::signature::generate_authorization;
use anyhow::Result;
use reqwest::Client;
use std::collections::HashMap;
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tracing::{debug, error, info};

/// 分块上传的阈值，超过此大小的文件将使用分块上传
const MULTIPART_THRESHOLD: u64 = 5 * 1024 * 1024; // 5 MB
/// 每个分块的大小
const PART_SIZE: u64 = 5 * 1024 * 1024; // 5 MB

pub struct Uploader {
    client: Client,
    config: Config,
}

pub type Metadata = HashMap<String, String>;

impl Uploader {
    /// 创建新的上传器实例
    ///
    /// # 参数
    ///
    /// * `config` - COS 配置
    pub fn new(config: Config) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    /// 上传文件到 COS
    ///
    /// 根据文件大小自动选择普通上传或分块上传
    ///
    /// # 参数
    ///
    /// * `file_path` - 要上传的文件路径
    /// * `object_key` - COS 中的对象键（存储路径）
    ///
    /// # 返回值
    ///
    /// 成功时返回上传后的文件 URL
    pub async fn upload_file<P: AsRef<Path>>(
        &self,
        file_path: P,
        object_key: &str,
        metadata: Option<Metadata>,
    ) -> Result<String> {
        let file_path = file_path.as_ref();
        let file_size = tokio::fs::metadata(file_path).await?.len();

        if file_size > MULTIPART_THRESHOLD {
            self.multipart_upload(file_path, object_key, metadata).await
        } else {
            self.simple_upload(file_path, object_key, metadata).await
        }
    }

    /// 普通上传
    async fn simple_upload<P: AsRef<Path>>(
        &self,
        file_path: P,
        object_key: &str,
        metadata: Option<Metadata>,
    ) -> Result<String> {
        let file_path = file_path.as_ref();
        debug!("普通上传文件: {:?}", file_path);

        let url = format!(
            "https://{}.cos.{}.myqcloud.com/{}",
            self.config.bucket, self.config.region, object_key
        );

        let content_type = mime_guess::from_path(file_path)
            .first_or_octet_stream()
            .to_string();

        let file_content = tokio::fs::read(file_path).await?;

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), content_type.clone());
        headers.insert(
            "Host".to_string(),
            format!(
                "{}.cos.{}.myqcloud.com",
                self.config.bucket, self.config.region
            ),
        );
        headers.insert("Content-Length".to_string(), file_content.len().to_string());

        // 添加元数据头
        if let Some(metadata) = metadata {
            for (key, value) in metadata {
                headers.insert(format!("x-cos-meta-{}", key), value);
            }
        }

        let params = HashMap::new();

        let authorization = generate_authorization(
            &self.config.secret_id,
            &self.config.secret_key,
            "put",
            &format!("/{}", object_key),
            &params,
            &headers,
            3600,
        );

        // 构建请求 headers
        let mut request = self
            .client
            .put(&url)
            .header("Authorization", authorization);

        for (key, value) in headers {
            request = request.header(key, value);
        }

        // 发送请求
        let response = request
            .body(file_content)
            .send()
            .await?;

        if response.status().is_success() {
            info!("文件上传成功: {}", url);
            Ok(url)
        } else {
            let error_message = response.text().await?;
            error!("文件上传失败: {}", error_message);
            Err(anyhow::anyhow!("上传失败: {}", error_message))
        }
    }

    /// 分块上传
    async fn multipart_upload<P: AsRef<Path>>(
        &self,
        file_path: P,
        object_key: &str,
        metadata: Option<Metadata>,
    ) -> Result<String> {
        let file_path = file_path.as_ref();
        info!("分块上传文件: {:?}", file_path);

        let base_url = format!(
            "https://{}.cos.{}.myqcloud.com/{}",
            self.config.bucket, self.config.region, object_key
        );

        // 初始化分块上传
        let upload_id = self.init_multipart_upload(object_key, metadata).await?;

        // 上传分块
        let mut file = File::open(file_path).await?;
        let file_size = file.metadata().await?.len();
        let mut part_number = 1u32;
        let mut etags = Vec::new();

        while (u64::from(part_number - 1)) * PART_SIZE < file_size {
            let start = u64::from(part_number - 1) * PART_SIZE;
            let end = std::cmp::min(u64::from(part_number) * PART_SIZE, file_size);
            let part_size = end - start;

            file.seek(std::io::SeekFrom::Start(start)).await?;
            let mut buffer = vec![0; part_size as usize];
            file.read_exact(&mut buffer).await?;

            let etag = self
                .upload_part(object_key, &upload_id, part_number, &buffer)
                .await?;
            etags.push((part_number, etag));

            part_number = part_number
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("分块编号溢出"))?;
        }

        // 完成分块上传
        self.complete_multipart_upload(object_key, &upload_id, &etags)
            .await?;

        Ok(base_url)
    }

    /// 初始化分块上传
    ///
    /// # 参数
    ///
    /// * `object_key` - COS 中的对象键（存储路径）
    ///
    /// # 返回值
    ///
    /// 成功时返回上传 ID
    async fn init_multipart_upload(
        &self,
        object_key: &str,
        metadata: Option<Metadata>,
    ) -> Result<String> {
        let url = format!(
            "https://{}.cos.{}.myqcloud.com/{}?uploads",
            self.config.bucket, self.config.region, object_key
        );

        let mut headers = HashMap::new();
        headers.insert(
            "Host".to_string(),
            format!(
                "{}.cos.{}.myqcloud.com",
                self.config.bucket, self.config.region
            ),
        );

        if let Some(metadata) = metadata {
            for (key, value) in metadata {
                headers.insert(format!("x-cos-meta-{}", key), value);
            }
        }

        let params = HashMap::from([("uploads".to_string(), "".to_string())]);

        let authorization = generate_authorization(
            &self.config.secret_id,
            &self.config.secret_key,
            "post",
            &format!("/{}", object_key),
            &params,
            &headers,
            3600,
        );

        // 构建请求 headers
        let mut request = self
            .client
            .post(&url)
            .header("Authorization", authorization);

        for (key, value) in headers {
            request = request.header(key, value);
        }

        let response = request
            .send()
            .await?;

        if response.status().is_success() {
            let text = response.text().await?;
            // 解析 XML 响应以获取 upload_id
            // 注意：这里使用了一个简单的字符串解析方法，在实际生产环境中应使用proper XML解析库
            let upload_id = text
                .split("<UploadId>")
                .nth(1)
                .unwrap()
                .split("</UploadId>")
                .next()
                .unwrap();
            Ok(upload_id.to_string())
        } else {
            Err(anyhow::anyhow!("初始化分块上传失败"))
        }
    }

    /// 上传单个分块
    ///
    /// # 参数
    ///
    /// * `object_key` - COS 中的对象键（存储路径）
    /// * `upload_id` - 初始化分块上传时返回的上传 ID
    /// * `part_number` - 分块的编号
    /// * `data` - 分块的数据
    ///
    /// # 返回值
    ///
    /// 成功时返回该分块的 ETag
    async fn upload_part(
        &self,
        object_key: &str,
        upload_id: &str,
        part_number: u32,
        data: &[u8],
    ) -> Result<String> {
        let url = format!(
            "https://{}.cos.{}.myqcloud.com/{}?partNumber={}&uploadId={}",
            self.config.bucket, self.config.region, object_key, part_number, upload_id
        );

        let mut headers = HashMap::new();
        headers.insert(
            "Host".to_string(),
            format!(
                "{}.cos.{}.myqcloud.com",
                self.config.bucket, self.config.region
            ),
        );
        headers.insert("Content-Length".to_string(), data.len().to_string());

        let params = HashMap::from([
            ("partNumber".to_string(), part_number.to_string()),
            ("uploadId".to_string(), upload_id.to_string()),
        ]);

        let authorization = generate_authorization(
            &self.config.secret_id,
            &self.config.secret_key,
            "put",
            &format!("/{}", object_key),
            &params,
            &headers,
            3600,
        );

        let response = self
            .client
            .put(&url)
            .header("Authorization", authorization)
            .header(
                "Host",
                format!(
                    "{}.cos.{}.myqcloud.com",
                    self.config.bucket, self.config.region
                ),
            )
            .body(data.to_vec())
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response
                .headers()
                .get("ETag")
                .unwrap()
                .to_str()
                .unwrap()
                .to_string())
        } else {
            Err(anyhow::anyhow!("上传分块失败"))
        }
    }

    /// 完成分块上传
    ///
    /// # 参数
    ///
    /// * `object_key` - COS 中的对象键（存储路径）
    /// * `upload_id` - 初始化分块上传时返回的上传 ID
    /// * `parts` - 已上传分块的信息，包含分块编号和对应的 ETag
    ///
    /// # 返回值
    ///
    /// 成功时返回 Ok(())
    async fn complete_multipart_upload(
        &self,
        object_key: &str,
        upload_id: &str,
        parts: &[(u32, String)],
    ) -> Result<()> {
        let url = format!(
            "https://{}.cos.{}.myqcloud.com/{}?uploadId={}",
            self.config.bucket, self.config.region, object_key, upload_id
        );

        let mut headers = HashMap::new();
        headers.insert(
            "Host".to_string(),
            format!(
                "{}.cos.{}.myqcloud.com",
                self.config.bucket, self.config.region
            ),
        );

        let params = HashMap::from([("uploadId".to_string(), upload_id.to_string())]);

        let authorization = generate_authorization(
            &self.config.secret_id,
            &self.config.secret_key,
            "post",
            &format!("/{}", object_key),
            &params,
            &headers,
            3600,
        );

        let body = format!(
            "<CompleteMultipartUpload>{}</CompleteMultipartUpload>",
            parts
                .iter()
                .map(|(part_number, etag)| format!(
                    "<Part><PartNumber>{}</PartNumber><ETag>{}</ETag></Part>",
                    part_number, etag
                ))
                .collect::<Vec<_>>()
                .join("")
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", authorization)
            .header(
                "Host",
                format!(
                    "{}.cos.{}.myqcloud.com",
                    self.config.bucket, self.config.region
                ),
            )
            .body(body)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("完成分块上传失败"))
        }
    }
}

// 为 Uploader 结构体实现一些辅助方法

impl Uploader {
    /// 获取对象的元数据
    ///
    /// # 参数
    ///
    /// * `object_key` - COS 中的对象键（存储路径）
    ///
    /// # 返回值
    ///
    /// 成功时返回对象的元数据
    pub async fn get_object_metadata(&self, object_key: &str) -> Result<HashMap<String, String>> {
        let url = format!(
            "https://{}.cos.{}.myqcloud.com/{}",
            self.config.bucket, self.config.region, object_key
        );

        let mut headers = HashMap::new();
        headers.insert(
            "Host".to_string(),
            format!(
                "{}.cos.{}.myqcloud.com",
                self.config.bucket, self.config.region
            ),
        );

        let params = HashMap::new();

        let authorization = generate_authorization(
            &self.config.secret_id,
            &self.config.secret_key,
            "head",
            &format!("/{}", object_key),
            &params,
            &headers,
            3600,
        );

        let response = self
            .client
            .head(&url)
            .header("Authorization", authorization)
            .header(
                "Host",
                format!(
                    "{}.cos.{}.myqcloud.com",
                    self.config.bucket, self.config.region
                ),
            )
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect())
        } else {
            Err(anyhow::anyhow!("获取对象元数据失败"))
        }
    }

    /// 删除对象
    ///
    /// # 参数
    ///
    /// * `object_key` - COS 中的对象键（存储路径）
    ///
    /// # 返回值
    ///
    /// 成功时返回 Ok(())
    pub async fn delete_object(&self, object_key: &str) -> Result<()> {
        let url = format!(
            "https://{}.cos.{}.myqcloud.com/{}",
            self.config.bucket, self.config.region, object_key
        );

        let mut headers = HashMap::new();
        headers.insert(
            "Host".to_string(),
            format!(
                "{}.cos.{}.myqcloud.com",
                self.config.bucket, self.config.region
            ),
        );

        let params = HashMap::new();

        let authorization = generate_authorization(
            &self.config.secret_id,
            &self.config.secret_key,
            "delete",
            &format!("/{}", object_key),
            &params,
            &headers,
            3600,
        );

        let response = self
            .client
            .delete(&url)
            .header("Authorization", authorization)
            .header(
                "Host",
                format!(
                    "{}.cos.{}.myqcloud.com",
                    self.config.bucket, self.config.region
                ),
            )
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("删除对象失败"))
        }
    }
}
