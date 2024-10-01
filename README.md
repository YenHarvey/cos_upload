# cos_upload

[![Crates.io](https://img.shields.io/crates/v/cos_upload.svg)](https://crates.io/crates/cos_upload)
[![Documentation](https://docs.rs/cos_upload/badge.svg)](https://docs.rs/cos_upload)
[![License](https://img.shields.io/crates/l/cos_upload.svg)](https://github.com/YenHarvey/cos_upload)

**注意：这是一个临时的用于上传文件到腾讯云对象存储（COS）的 Rust 库。**

`cos_upload` 是一个简单的 Rust 库，用于将文件上传到腾讯云对象存储（COS）。它提供了简单和分块上传的功能，可以根据文件大小自动选择合适的上传方式。

## 功能

- 支持普通上传和分块上传
- 自动根据文件大小选择上传方式
- 支持获取对象元数据
- 支持删除对象

## 安装

将以下行添加到你的 `Cargo.toml` 文件中：

```toml
[dependencies]
cos_upload = "0.1.0"
```

## 使用示例

### 环境变量

```bash
TENCENT_SECRET_ID=
TENCENT_SECRET_KEY=

TENCENT_COS_REGION=
TENCENT_COS_BUCKET=
```

### 代码示例

```rust
use cos_upload::{Config, Uploader};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // 从环境变量创建配置
    let config = Config::from_env()?;

    // 创建上传器
    let uploader = Uploader::new(config);

    // 上传文件
    let file_path = "path/to/your/file.jpg";
    let object_key = "uploads/file.jpg";

    match uploader.upload_file(file_path, object_key).await {
        Ok(url) => println!("文件上传成功。URL: {}", url),
        Err(e) => eprintln!("文件上传失败: {}", e),
    }

    Ok(())
}
```

更多详细示例和用法，请参阅 [文档](https://docs.rs/cos_upload)。

## 注意事项

1. 这是一个临时的库，可能不适合在生产环境中使用。
2. 使用前请确保您有有效的腾讯云 COS 账户和相应的权限。
3. 请妥善保管您的 SecretId 和 SecretKey，不要将其硬编码在代码中或提交到版本控制系统。

## 许可证

根据 MIT 许可证或 Apache License 2.0 许可证（由您选择）授权。
