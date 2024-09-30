use chrono::Utc;
use hmac::{Hmac, Mac};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use urlencoding::encode as url_encode;

/// 生成腾讯云 COS 的授权签名
///
/// # 参数
///
/// * `secret_id` - 腾讯云 SecretId
/// * `secret_key` - 腾讯云 SecretKey
/// * `method` - HTTP 方法（如 "get", "put", "post" 等）
/// * `path` - 对象的路径
/// * `params` - 查询参数
/// * `headers` - HTTP 头部
/// * `expire` - 签名的有效期（以秒为单位）
///
/// # 返回值
///
/// 返回生成的授权签名字符串
pub(crate) fn generate_authorization(
    secret_id: &str,
    secret_key: &str,
    method: &str,
    path: &str,
    params: &HashMap<String, String>,
    headers: &HashMap<String, String>,
    expire: i64,
) -> String {
    let now = Utc::now();
    let start_time = now.timestamp();
    let end_time = start_time + expire;
    let key_time = format!("{};{}", start_time, end_time);

    let (url_param_list, http_parameters) = format_params(params);
    let (header_list, http_headers) = format_headers(headers);

    let sign_key = hmac_sha1(secret_key, &key_time);

    let http_string = format!(
        "{}\n{}\n{}\n{}\n",
        method.to_lowercase(),
        path,
        http_parameters,
        http_headers
    );

    let sha1_http_string = sha1_digest(&http_string);
    let string_to_sign = format!("sha1\n{}\n{}\n", key_time, sha1_http_string);

    let signature = hmac_sha1(&sign_key, &string_to_sign);

    format!(
        "q-sign-algorithm=sha1&q-ak={}&q-sign-time={}&q-key-time={}&q-header-list={}&q-url-param-list={}&q-signature={}",
        secret_id, key_time, key_time, header_list, url_param_list, signature
    )
}

fn format_params(params: &HashMap<String, String>) -> (String, String) {
    let mut sorted_params: Vec<_> = params.iter().collect();
    sorted_params.sort_by(|a, b| a.0.cmp(b.0));

    let url_param_list = sorted_params
        .iter()
        .map(|(k, _)| k.to_lowercase())
        .collect::<Vec<_>>()
        .join(";");

    let http_parameters = sorted_params
        .iter()
        .map(|(k, v)| format!("{}={}", url_encode(&k.to_lowercase()), url_encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    (url_param_list, http_parameters)
}

fn format_headers(headers: &HashMap<String, String>) -> (String, String) {
    let mut sorted_headers: Vec<_> = headers.iter().collect();
    sorted_headers.sort_by(|a, b| a.0.cmp(b.0));

    let header_list = sorted_headers
        .iter()
        .map(|(k, _)| k.to_lowercase())
        .collect::<Vec<_>>()
        .join(";");

    let http_headers = sorted_headers
        .iter()
        .map(|(k, v)| format!("{}={}", url_encode(&k.to_lowercase()), url_encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    (header_list, http_headers)
}

fn hmac_sha1(key: &str, message: &str) -> String {
    let mut mac = Hmac::<Sha1>::new_from_slice(key.as_bytes()).unwrap();
    mac.update(message.as_bytes());
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}

fn sha1_digest(message: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(message.as_bytes());
    hex::encode(hasher.finalize())
}
