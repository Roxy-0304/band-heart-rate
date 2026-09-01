use serde::Deserialize;

/// GitHub Release 响应结构
#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

/// 版本检查结果
pub struct VersionCheckResult {
    pub latest_version: Option<String>,
    pub error: Option<String>,
}

/// 检查 GitHub 最新版本
pub async fn check_latest_version() -> VersionCheckResult {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return VersionCheckResult {
                latest_version: None,
                error: Some(format!("创建请求失败: {e}")),
            }
        }
    };

    let url =
        "https://api.github.com/repos/Roxy-0304/band-heart-rate/releases/latest";

    let resp = match client
        .get(url)
        .header("User-Agent", "band-heart-rate")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return VersionCheckResult {
                latest_version: None,
                error: Some(format!("网络请求失败: {e}")),
            }
        }
    };

    let release: GitHubRelease = match resp.json().await {
        Ok(r) => r,
        Err(e) => {
            return VersionCheckResult {
                latest_version: None,
                error: Some(format!("解析响应失败: {e}")),
            }
        }
    };

    let version = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name)
        .to_string();

    VersionCheckResult {
        latest_version: Some(version),
        error: None,
    }
}

/// 比较版本号，返回 true 如果 latest > current
pub fn is_newer_version(latest: &str, current: &str) -> bool {
    let parse_version = |v: &str| -> Vec<u32> {
        v.split('.')
            .filter_map(|s| s.parse().ok())
            .collect()
    };

    let latest_parts = parse_version(latest);
    let current_parts = parse_version(current);

    latest_parts > current_parts
}
