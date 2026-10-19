use crate::{
    accounts::Account,
    network::{self, Transport},
};
use serde_json::{json, Value};
use url::Url;

pub const ENTRY: &str = "https://jwxt.xjtu.edu.cn/jwapp/sys/homeapp/index.do";
pub const BASE: &str = "https://jwxt.xjtu.edu.cn";
pub struct Client {
    pub transport: Transport,
    pub account: Account,
}
impl Client {
    pub async fn connect(account: Account) -> Result<Self, String> {
        if account.degree != "undergraduate" {
            return Err("此接口目前支持本科教务系统，研究生适配尚未完成".into());
        }
        let this = Self {
            transport: Transport::new(Some(&account.cookies))?,
            account,
        };
        let page = this
            .transport
            .page(
                &Url::parse(ENTRY).unwrap(),
                this.account.mode,
                &this.account.degree,
            )
            .await?;
        if page.url.host_str() != Some("jwxt.xjtu.edu.cn")
            || page.html.contains("name=\"execution\"")
        {
            return Err("教务登录态已失效，请在校园账号中重新登录后再试".into());
        }
        let user = this
            .get("/jwapp/sys/homeapp/api/home/currentUser.do")
            .await?;
        // The session must identify an actual user; a generic 200 login HTML is not success.
        if !user.get("datas").is_some_and(Value::is_object) {
            return Err("教务系统未返回已登录用户".into());
        }
        Ok(this)
    }
    pub async fn initialize_courses(&self) -> Result<(), String> {
        let page = self
            .transport
            .page(
                &Url::parse("https://jwxt.xjtu.edu.cn/jwapp/sys/kcbcx/*default/index.do").unwrap(),
                self.account.mode,
                &self.account.degree,
            )
            .await?;
        if page.url.host_str() != Some("jwxt.xjtu.edu.cn") {
            return Err("全校课程服务要求重新认证".into());
        }
        Ok(())
    }
    fn url(&self, path: &str) -> Result<Url, String> {
        if !path.starts_with("/jwapp/") || path.contains("..") {
            return Err("教务接口路径无效".into());
        }
        let logical = Url::parse(&format!("{BASE}{path}")).map_err(|_| "教务接口地址无效")?;
        Transport::address(&logical, self.account.mode)
    }
    pub async fn get(&self, path: &str) -> Result<Value, String> {
        let response = self
            .transport
            .client
            .get(self.url(path)?)
            .header("Referer", ENTRY)
            .send()
            .await
            .map_err(|_| "教务服务连接失败")?;
        decode(response).await
    }
    pub async fn post(&self, path: &str, data: &[(String, String)]) -> Result<Value, String> {
        let response = self
            .transport
            .client
            .post(self.url(path)?)
            .header("Referer", ENTRY)
            .header("X-Requested-With", "XMLHttpRequest")
            .form(data)
            .send()
            .await
            .map_err(|_| "教务请求未收到响应，请刷新状态后确认结果")?;
        decode(response).await
    }
    pub async fn rows(
        &self,
        path: &str,
        table: &str,
        data: &[(String, String)],
    ) -> Result<Vec<Value>, String> {
        rows(&self.post(path, data).await?, table)
    }
    pub async fn paged(
        &self,
        path: &str,
        table: &str,
        data: &[(String, String)],
    ) -> Result<Vec<Value>, String> {
        let mut result = Vec::new();
        let mut previous = None;
        for page in 1..=100 {
            let mut fields = data.to_vec();
            fields.extend([
                ("pageSize".into(), "100".into()),
                ("pageNumber".into(), page.to_string()),
            ]);
            let value = self.post(path, &fields).await?;
            let batch = rows(&value, table)?;
            let total = value["datas"][table]["totalSize"].as_u64().or_else(|| {
                value["datas"][table]["totalSize"]
                    .as_str()
                    .and_then(|v| v.parse().ok())
            });
            let fingerprint = serde_json::to_string(&batch).map_err(|_| "教务分页格式无效")?;
            if previous.as_ref() == Some(&fingerprint) && !batch.is_empty() {
                return Err("教务系统重复返回同一页，已停止查询".into());
            }
            previous = Some(fingerprint);
            let count = batch.len();
            result.extend(batch);
            if total.is_some_and(|n| result.len() as u64 >= n)
                || count == 0
                || (total.is_none() && count < 100)
            {
                return Ok(result);
            }
        }
        Err("教务数据超出单次查询范围，请缩小学期或课程范围".into())
    }
    pub async fn student_role(&self) -> Result<(), String> {
        let user = self
            .get("/jwapp/sys/homeapp/api/home/currentUser.do")
            .await?;
        let roles = user["datas"]["userGroups"]
            .as_array()
            .ok_or("教务身份列表格式变化")?;
        let student = roles
            .iter()
            .find(|r| r["roleName"] == "学生")
            .ok_or("当前账号没有教务学生身份")?;
        if student["currentRole"] != true {
            let id = required(student, "roleId")?;
            self.post(
                "/jwapp/sys/homeapp/api/home/changeAppRole.do",
                &[("appRole".into(), id)],
            )
            .await?;
        }
        Ok(())
    }
    pub async fn term(&self) -> Result<String, String> {
        let result = self
            .rows("/jwapp/sys/wdkb/modules/jshkcb/dqxnxq.do", "dqxnxq", &[])
            .await?;
        required(result.first().ok_or("教务系统未提供当前学期")?, "DM")
    }
}
pub fn success(value: &Value) -> bool {
    value == "0" || value == 0
}
async fn decode(response: reqwest::Response) -> Result<Value, String> {
    if response.status().is_redirection() || [401, 403].contains(&response.status().as_u16()) {
        return Err("教务登录态已失效，请重新登录".into());
    }
    if !response.status().is_success() {
        return Err(format!(
            "教务服务暂不可用（HTTP {}）",
            response.status().as_u16()
        ));
    }
    let bytes = network::limited_bytes(response, 8 * 1024 * 1024).await?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|_| "教务返回了登录页或无法识别的数据，请重新登录后重试")?;
    if !success(&value["code"]) {
        return Err("教务系统拒绝了请求，请检查登录状态或业务开放时间".into());
    }
    Ok(value)
}
pub fn rows(value: &Value, table: &str) -> Result<Vec<Value>, String> {
    if !success(&value["code"]) {
        return Err("教务请求未成功".into());
    }
    value["datas"][table]["rows"]
        .as_array()
        .cloned()
        .ok_or_else(|| format!("教务响应缺少 {table} 数据列表"))
}
pub fn field(value: &Value, name: &str) -> String {
    match &value[name] {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}
pub fn required(value: &Value, name: &str) -> Result<String, String> {
    let text = field(value, name);
    if text.is_empty() || text.len() > 2048 {
        Err(format!("教务响应字段 {name} 无效"))
    } else {
        Ok(text)
    }
}
pub fn condition(name: &str, value: &str) -> Value {
    json!({"name":name,"value":value,"builder":"equal","linkOpt":"AND"})
}
