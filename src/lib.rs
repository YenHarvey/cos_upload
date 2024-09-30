//! # COS上传库
//!
//! `cos_upload` 是一个用于临时上传文件到腾讯云对象存储（COS）的 Rust 库。
//! 它支持普通上传和分块上传，可以根据文件大小(5MB Default)自动选择合适的上传方式。
//!
//! ## 示例
//!
//! 以下是一个基本的使用示例：
//!
//! ```rust
//! use cos_upload::{Config, Uploader};
//! use dotenv::dotenv;
//! use anyhow::Result;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     dotenv().ok();
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
//!     // 创建上传器
//!     let uploader = Uploader::new(config);
//!
//!     // 上传文件
//!     let file_path = "path/to/your/file.jpg";
//!     let object_key = "uploads/file.jpg";
//!
//!     match uploader.upload_file(file_path, object_key).await {
//!         Ok(url) => println!("文件上传成功。URL: {}", url),
//!         Err(e) => eprintln!("文件上传失败: {}", e),
//!     }
//!
//!     // 获取对象元数据
//!     match uploader.get_object_metadata(object_key).await {
//!         Ok(metadata) => println!("对象元数据: {:?}", metadata),
//!         Err(e) => eprintln!("获取元数据失败: {}", e),
//!     }
//!
//!     // 删除对象
//!     match uploader.delete_object(object_key).await {
//!         Ok(_) => println!("对象删除成功"),
//!         Err(e) => eprintln!("对象删除失败: {}", e),
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! 注意：在运行此示例之前，请确保设置了正确的环境变量或在代码中提供了正确的配置信息。

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
        let upload_result = uploader.upload_file(&file_path, object_key).await;
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
