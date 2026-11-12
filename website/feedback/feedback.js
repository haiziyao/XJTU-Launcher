// 反馈表单：字符计数、Bug 才显示复现步骤、提交到 site-api 的 POST /api/feedback。
// 接口路径按 document.baseURI 解析：IP 预览在 /site/api/feedback，域名上线后是 /api/feedback。
(function () {
  "use strict";

  var form = document.getElementById("fb-form");
  if (!form) return;

  var title = document.getElementById("fb-title");
  var body = document.getElementById("fb-body");
  var steps = document.getElementById("fb-steps");
  var stepsField = document.getElementById("fb-steps-field");
  var contact = document.getElementById("fb-contact");
  var consent = document.getElementById("fb-consent");
  var honeypot = document.getElementById("fb-website");
  var submit = document.getElementById("fb-submit");
  var error = document.getElementById("fb-error");
  var success = document.getElementById("fb-success");
  var successCode = document.getElementById("fb-success-code");
  var apiUrl = new URL("../api/feedback", document.baseURI).href;

  var MESSAGES = {
    title: "标题写 4–60 个字。",
    body: "详细描述至少写 10 个字。",
    kind: "先选一个类型。",
    consent: "先勾选下面的说明再提交。",
    contact: "联系方式太长了。",
    steps: "复现步骤太长了。",
    spam: "提交被拦截了。如果这是误判，去 GitHub Issue 说一声。",
    "too many": "提交太频繁，10 分钟后再试。",
    "too large": "内容太长，提交不了。",
    storage: "服务器没保存成功，稍后再试。内容还在。",
    json: "提交的数据没发出去，刷新页面后再试。内容会丢，先复制一下。",
    network: "网络没连上，稍后再试。内容还在。"
  };

  function showError(message, field) {
    error.textContent = message;
    error.hidden = false;
    if (field && field.focus) field.focus();
  }

  function clearError() {
    error.hidden = true;
    error.textContent = "";
  }

  function count(input, out) {
    var update = function () {
      out.textContent = input.value.length;
    };
    input.addEventListener("input", update);
    update();
  }

  count(title, document.getElementById("count-title"));
  count(body, document.getElementById("count-body"));

  form.querySelectorAll('input[name="kind"]').forEach(function (radio) {
    radio.addEventListener("change", function () {
      if (radio.checked) stepsField.hidden = radio.value !== "bug";
    });
  });

  function selectedKind() {
    var checked = form.querySelector('input[name="kind"]:checked');
    return checked ? checked.value : "idea";
  }

  function validate() {
    var titleText = title.value.trim();
    var bodyText = body.value.trim();
    if (titleText.length < 4 || titleText.length > 60) {
      return { message: MESSAGES.title, field: title };
    }
    if (bodyText.length < 10 || bodyText.length > 2000) {
      return { message: MESSAGES.body, field: body };
    }
    if (!consent.checked) {
      return { message: MESSAGES.consent, field: consent };
    }
    return null;
  }

  form.addEventListener("submit", function (event) {
    event.preventDefault();
    clearError();

    var problem = validate();
    if (problem) {
      showError(problem.message, problem.field);
      return;
    }

    submit.disabled = true;
    submit.textContent = "正在提交…";

    var data = {
      kind: selectedKind(),
      title: title.value.trim(),
      body: body.value.trim(),
      steps: selectedKind() === "bug" ? steps.value.trim() : "",
      contact: contact.value.trim(),
      consent: true,
      website: honeypot.value
    };

    fetch(apiUrl, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data)
    })
      .then(function (response) {
        return response
          .json()
          .catch(function () {
            return {};
          })
          .then(function (result) {
            return { status: response.status, result: result };
          });
      })
      .then(function (outcome) {
        if (outcome.status === 200 && outcome.result.ok) {
          successCode.textContent = outcome.result.id;
          form.hidden = true;
          success.hidden = false;
          success.scrollIntoView({ block: "center" });
          return;
        }
        var code = outcome.result && outcome.result.error;
        showError(MESSAGES[code] || MESSAGES.network);
      })
      .catch(function () {
        showError(MESSAGES.network);
      })
      .then(function () {
        submit.disabled = false;
        submit.textContent = "提交反馈";
      });
  });
})();
