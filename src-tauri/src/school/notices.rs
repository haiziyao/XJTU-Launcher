//! Public campus notice boards. No account or cookie is involved: the channel
//! list is closed, every page is HTTPS on an `xjtu.edu.cn` host, and only the
//! title, category, date and link are returned.
use crate::network::campus_web_url;
use scraper::{Html, Selector};
use serde_json::{json, Value};
use std::{sync::OnceLock, time::Duration};
use url::Url;

/// Notice pages are simple list pages; anything larger is not a list we know.
const MAX_PAGE_BYTES: usize = 2 * 1024 * 1024;
const MAX_LIMIT: u32 = 50;
const MAX_PARSE: usize = 100;

pub struct Channel {
    pub id: &'static str,
    pub name: &'static str,
    pub level: &'static str,
    pub url: &'static str,
}

/// A closed list keeps the Agent from turning this tool into an open web fetch.
pub const CHANNELS: &[Channel] = &[
    Channel {
        id: "dean",
        name: "教务处教学通知",
        level: "undergraduate",
        url: "https://dean.xjtu.edu.cn/jxxx/jxtz2.htm",
    },
    Channel {
        id: "dean-exam",
        name: "教务处考试安排",
        level: "undergraduate",
        url: "https://dean.xjtu.edu.cn/jxxx/jxtz2/ksap.htm",
    },
    Channel {
        id: "dean-course",
        name: "教务处课程安排",
        level: "undergraduate",
        url: "https://dean.xjtu.edu.cn/jxxx/jxtz2/kcap.htm",
    },
    Channel {
        id: "dean-competition",
        name: "教务处竞赛大创",
        level: "undergraduate",
        url: "https://dean.xjtu.edu.cn/jxxx/jxtz2/jsdc.htm",
    },
    Channel {
        id: "xsc",
        name: "学生处通知公告",
        level: "undergraduate",
        url: "https://xsc.xjtu.edu.cn/xgdt/tzgg.htm",
    },
    Channel {
        id: "gs",
        name: "研究生院培养工作",
        level: "graduate",
        url: "https://gs.xjtu.edu.cn/tzgg/pygz.htm",
    },
    Channel {
        id: "nic",
        name: "网络信息中心通知公告",
        level: "all",
        url: "https://nic.xjtu.edu.cn/xwzx/tzgg.htm",
    },
];

pub fn channels() -> Value {
    json!({
        "channels": CHANNELS
            .iter()
            .map(|channel| json!({
                "id": channel.id,
                "name": channel.name,
                "level": channel.level,
            }))
            .collect::<Vec<_>>()
    })
}

pub fn validate(channel: &str, limit: u32) -> Result<(), String> {
    if find(channel).is_none() {
        return Err("请选择有效的通知栏目".into());
    }
    if !(1..=MAX_LIMIT).contains(&limit) {
        return Err(format!("每次最多读取 {MAX_LIMIT} 条通知"));
    }
    Ok(())
}

fn find(id: &str) -> Option<&'static Channel> {
    CHANNELS.iter().find(|channel| channel.id == id)
}

pub async fn list(channel: &str, limit: u32) -> Result<Value, String> {
    validate(channel, limit)?;
    let channel = find(channel).ok_or("请选择有效的通知栏目")?;
    let page = Url::parse(channel.url).map_err(|_| "通知栏目地址无效")?;
    if !campus_web_url(&page) {
        return Err("通知栏目地址不受支持".into());
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(crate::network::USER_AGENT)
        .build()
        .map_err(|_| "无法创建通知连接")?;
    let mut response = client
        .get(page.clone())
        .send()
        .await
        .map_err(|_| "无法连接通知栏目，请检查网络后重试")?;
    if !response.status().is_success() {
        return Err(format!(
            "通知栏目暂不可用（HTTP {}）",
            response.status().as_u16()
        ));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "读取通知栏目时网络中断")?
    {
        if body.len() + chunk.len() > MAX_PAGE_BYTES {
            return Err("通知栏目页面过大，已停止读取".into());
        }
        body.extend_from_slice(&chunk);
    }
    let html = String::from_utf8_lossy(&body);
    // Parse more than requested so pinned or out-of-order entries cannot hide
    // a newer notice, then sort by date and keep the requested number.
    let mut items = parse(&page, &html, MAX_PARSE);
    if items.is_empty() {
        return Err("没有从该栏目读取到通知，页面结构可能已变化".into());
    }
    items.sort_by(|left, right| {
        right["date"]
            .as_str()
            .unwrap_or_default()
            .cmp(left["date"].as_str().unwrap_or_default())
    });
    items.truncate(limit as usize);
    Ok(json!({
        "channel": channel.id,
        "name": channel.name,
        "url": channel.url,
        "items": items,
    }))
}

fn parse(page: &Url, html: &str, limit: usize) -> Vec<Value> {
    let document = Html::parse_document(html);
    let row = Selector::parse("li").expect("selector");
    let link = Selector::parse("a[href]").expect("selector");
    let heading = Selector::parse("h2").expect("selector");
    let span = Selector::parse("span").expect("selector");
    let mut items = Vec::new();
    let mut seen: Vec<Url> = Vec::new();
    for entry in document.select(&row) {
        let Some(anchor) = entry
            .select(&link)
            .find(|anchor| anchor.value().attr("href").is_some_and(article_link))
        else {
            continue;
        };
        let href = anchor.value().attr("href").unwrap_or_default();
        let Ok(target) = page.join(href) else {
            continue;
        };
        if !campus_web_url(&target) || seen.contains(&target) {
            continue;
        }
        let raw = collapse(&anchor.text().collect::<String>());
        let (category, bare) = strip_category(&raw);
        let title = anchor
            .select(&heading)
            .next()
            .map(|heading| collapse(&heading.text().collect::<String>()))
            .filter(|title| !title.is_empty())
            .or_else(|| {
                anchor
                    .value()
                    .attr("title")
                    .map(collapse)
                    .filter(|title| !title.is_empty())
            })
            .unwrap_or(bare);
        if title.is_empty() {
            continue;
        }
        let date = entry
            .select(&span)
            .find_map(|span| span_date(&collapse(&span.text().collect::<String>())))
            .or_else(|| find_date(&entry.text().collect::<String>()))
            .unwrap_or_default();
        seen.push(target.clone());
        items.push(json!({
            "title": title,
            "category": category,
            "date": date,
            "url": target.to_string(),
        }));
        if items.len() >= limit {
            break;
        }
    }
    items
}

/// Article links on the school CMS point at `/info/...` or a `content.jsp`
/// query; this keeps navigation, pagination and footer links out.
fn article_link(href: &str) -> bool {
    href.contains("/info/") || href.contains("wbnewsid=") || href.starts_with("content.jsp")
}

fn strip_category(raw: &str) -> (String, String) {
    if let Some(rest) = raw.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            return (
                rest[..end].trim().to_string(),
                rest[end + 1..].trim().to_string(),
            );
        }
    }
    (String::new(), raw.trim().to_string())
}

fn collapse(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('\u{200b}', "")
        .trim()
        .to_string()
}

fn date_regex() -> &'static regex::Regex {
    static DATE: OnceLock<regex::Regex> = OnceLock::new();
    DATE.get_or_init(|| {
        regex::Regex::new(r"(\d{4})[-/年](\d{1,2})[-/月](\d{1,2})").expect("date regex")
    })
}

fn dashes_regex() -> &'static regex::Regex {
    static DASHES: OnceLock<regex::Regex> = OnceLock::new();
    DASHES.get_or_init(|| {
        // The day and month may be separate elements with no space between
        // them once the text nodes are concatenated ("252026/06").
        regex::Regex::new(r"^(\d{1,2})\s*(\d{4})[-/](\d{1,2})$").expect("date regex")
    })
}

fn find_date(text: &str) -> Option<String> {
    let found = date_regex().captures(text)?;
    let year: u32 = found.get(1)?.as_str().parse().ok()?;
    let month: u32 = found.get(2)?.as_str().parse().ok()?;
    let day: u32 = found.get(3)?.as_str().parse().ok()?;
    date(year, month, day)
}

/// Some list templates split the day from the month, for example a card that
/// reads "25 2026/06".
fn span_date(text: &str) -> Option<String> {
    if let Some(date) = find_date(text) {
        return Some(date);
    }
    let found = dashes_regex().captures(text)?;
    let day: u32 = found.get(1)?.as_str().parse().ok()?;
    let year: u32 = found.get(2)?.as_str().parse().ok()?;
    let month: u32 = found.get(3)?.as_str().parse().ok()?;
    date(year, month, day)
}

fn date(year: u32, month: u32, day: u32) -> Option<String> {
    if !(2000..=2200).contains(&year) || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(format!("{year:04}-{month:02}-{day:02}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_page(html: &str, limit: usize) -> Vec<Value> {
        parse(
            &Url::parse("https://dean.xjtu.edu.cn/jxxx/jxtz2.htm").expect("page"),
            html,
            limit,
        )
    }

    #[test]
    fn dean_lists_keep_category_title_and_date() {
        let items = parse_page(
            r#"<ul class="list">
                <li><a href="../info/1176/10470.htm"><i>[培养方案]</i>关于劳动教育的通知</a><span>2026-09-18</span></li>
                <li><a href="jxtz2/624.htm">2</a></li>
            </ul>"#,
            10,
        );
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["title"], "关于劳动教育的通知");
        assert_eq!(items[0]["category"], "培养方案");
        assert_eq!(items[0]["date"], "2026-09-18");
        assert_eq!(
            items[0]["url"],
            "https://dean.xjtu.edu.cn/info/1176/10470.htm"
        );
    }

    #[test]
    fn student_affairs_day_month_split_is_understood() {
        let items = parse_page(
            r#"<ul><li>
                <span><b>25</b><i>2026/06</i></span>
                <a href="../info/1060/16675.htm" title="勤工助学通知">
                  <h2>关于开展勤工助学岗位设置的通知</h2>
                  <p>正文里还有一个 2024年 的旧日期，不应被采用。</p>
                </a>
            </li></ul>"#,
            10,
        );
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["title"], "关于开展勤工助学岗位设置的通知");
        assert_eq!(items[0]["date"], "2026-06-25");
    }

    #[test]
    fn notices_stop_at_the_limit_and_skip_foreign_hosts() {
        let html = r#"<ul>
            <li><span class="time fr">2026-10-20</span><a href="../info/1146/1.htm">第一条</a></li>
            <li><span class="time fr">2026-10-19</span><a href="https://example.test/info/1146/2.htm">外站</a></li>
            <li><span class="time fr">2026-10-18</span><a href="../info/1146/3.htm">第三条</a></li>
        </ul>"#;
        let items = parse_page(html, 1);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["title"], "第一条");
        let all = parse_page(html, 10);
        assert_eq!(all.len(), 2);
    }

    /// Manual probe: reads every public channel once. It prints only counts,
    /// dates and titles from public pages.
    #[tokio::test]
    #[ignore = "live probe; reads public notice pages"]
    async fn live_notice_channels_probe() {
        for channel in CHANNELS {
            match list(channel.id, 5).await {
                Ok(value) => {
                    let items = value["items"].as_array().cloned().unwrap_or_default();
                    println!(
                        "{} items={} first={} {}",
                        channel.id,
                        items.len(),
                        items
                            .first()
                            .map(|item| item["date"].as_str().unwrap_or(""))
                            .unwrap_or(""),
                        items
                            .first()
                            .map(|item| item["title"].as_str().unwrap_or(""))
                            .unwrap_or("")
                    );
                }
                Err(error) => println!("{} error: {error}", channel.id),
            }
        }
    }

    #[test]
    fn notice_parameters_are_closed_and_bounded() {
        assert!(validate("dean", 10).is_ok());
        assert!(validate("https://example.test", 10).is_err());
        assert!(validate("dean", 0).is_err());
        assert!(validate("dean", MAX_LIMIT + 1).is_err());
    }
}
