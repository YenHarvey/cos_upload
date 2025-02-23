use anyhow::Result;

/// COS 配置结构体
#[derive(Debug, Clone)]
pub struct Config {
    /// 腾讯云 SecretId
    pub secret_id: String,
    /// 腾讯云 SecretKey
    pub secret_key: String,
    /// COS 地域
    pub region: String,
    /// COS Bucket 名称
    pub bucket: String,
}

impl Config {
    /// 从环境变量创建新的配置
    ///
    /// 需要设置以下环境变量：
    /// - TENCENT_SECRET_ID
    /// - TENCENT_SECRET_KEY
    /// - TENCENT_COS_REGION
    /// - TENCENT_COS_BUCKET
    ///
    /// # 错误
    ///
    /// 如果任何必需的环境变量未设置，将返回错误。
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            secret_id: std::env::var("TENCENT_SECRET_ID")?,
            secret_key: std::env::var("TENCENT_SECRET_KEY")?,
            region: std::env::var("TENCENT_COS_REGION")?,
            bucket: std::env::var("TENCENT_COS_BUCKET")?,
        })
    }

    /// 手动创建新的配置
    pub fn new(secret_id: String, secret_key: String, region: String, bucket: String) -> Self {
        Self {
            secret_id,
            secret_key,
            region,
            bucket,
        }
    }
}
