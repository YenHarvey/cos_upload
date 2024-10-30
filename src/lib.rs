// Copyright 2024 Yen Harvey
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # COS 上传库
//!
//! `cos_upload` 是一个用于将文件临时上传到腾讯云对象存储（COS）的 Rust 库。
//! 支持普通上传和分块上传，可以根据文件大小（默认 5MB）自动选择合适的上传方式。
//!
//! ## 功能亮点
//! - 支持普通上传和分块上传，根据文件大小自动选择
//! - 支持自定义对象元数据（例如用户 ID、用户名、上传时间等）
//!
//! ## 示例
//!
//! 以下是一个基本的使用示例，展示了如何上传文件，并附带自定义的元数据。
//!
//! ```rust
//! use anyhow::Result;
//! use chrono::Utc;
//! use cos_upload::{Config, Uploader};
//! use std::collections::HashMap;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     dotenv::dotenv().ok();
//!
//!     // 从环境变量创建配置
//!     let config = Config::from_env()?;
//!
//!     // 或者手动创建配置
//!     // let config = Config::new(
//!     //     "secret_id".to_string(),
//!     //     "secret_key".to_string(),
//!     //     "region".to_string(),
//!     //     "bucket".to_string()
//!     // );
//!
//!     // 创建上传器实例
//!     let uploader = Uploader::new(config);
//!
//!     // 创建并添加对象元数据
//!     let mut metadata = HashMap::new();
//!     metadata.insert("user-id".to_string(), "123".to_string());
//!     metadata.insert("username".to_string(), "sample_user".to_string());
//!     metadata.insert("source".to_string(), "sample_source".to_string());
//!     metadata.insert("upload-time".to_string(), Utc::now().to_rfc3339());
//!
//!     // 上传文件，使用通用文件路径示例
//!     let file_path = "path/to/local/testfile";
//!     let object_key = "uploads/user_123/sample_file"; // 按用户组织路径
//!
//!     match uploader.upload_file(file_path, object_key, Some(metadata)).await {
//!         Ok(url) => println!("文件上传成功。URL: {}", url),
//!         Err(e) => eprintln!("文件上传失败: {}", e),
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## 注意事项
//! - 在运行此示例之前，请确保设置了正确的环境变量或在代码中提供了准确的配置数据。
//! - 使用 `metadata` 字典来存储和传递自定义的元数据信息，这些信息将附加到上传的对象中，便于后续查询。
//! - 文件路径和对象键（`object_key`）可以根据业务需求自定义，例如按用户 ID 组织的路径结构，以更好地管理上传的资源。

mod config;
mod signature;
mod uploader;

pub use config::Config;
pub use uploader::Uploader;

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[tokio::test]
    async fn test_upload_and_delete() {
        // 设置测试环境变量
        // env::set_var("TENCENT_SECRET_ID", "");
        // env::set_var("TENCENT_SECRET_KEY", "");
        // env::set_var("TENCENT_COS_REGION", "");
        // env::set_var("TENCENT_COS_BUCKET", "");
        dotenv::dotenv().ok();

        let config = Config::from_env().expect("Failed to load config from env");
        let uploader = Uploader::new(config);

        // 创建一个临时文件用于测试
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("test_file.txt");
        std::fs::write(&file_path, "Hello, Tencent COS!").expect("Failed to write test file");

        let object_key = "test/test_file.txt";

        // 测试上传
        let upload_result = uploader.upload_file(&file_path, object_key, None).await;
        assert!(
            upload_result.is_ok(),
            "Upload failed: {:?}",
            upload_result.err()
        );

        // 测试获取元数据
        let metadata_result = uploader.get_object_metadata(object_key).await;
        assert!(
            metadata_result.is_ok(),
            "Get metadata failed: {:?}",
            metadata_result.err()
        );

        // 测试删除
        let delete_result = uploader.delete_object(object_key).await;
        assert!(
            delete_result.is_ok(),
            "Delete failed: {:?}",
            delete_result.err()
        );

        // 清理临时文件
        temp_dir.close().expect("Failed to delete temp dir");
    }
}
