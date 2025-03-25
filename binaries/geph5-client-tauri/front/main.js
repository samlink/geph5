const { invoke } = window.__TAURI__.core;

// 登录页面元素
const loginForm = document.querySelector("#login");
const mainContent = document.querySelector("#main");
const loginBtn = document.querySelector("#login-btn");
const usernameInput = document.querySelector("#username");
const passwordInput = document.querySelector("#password");
const loginMessage = document.querySelector("#login-message");

// 主页面元素
const connectBtn = document.querySelector("#connect-btn");
const disconnectBtn = document.querySelector("#disconnect-btn");
const vpnCheck = document.querySelector("#all-proxy");
const stat = document.querySelector("#status");
const messagep = document.querySelector(".msg p");

window.onload = function () {
  is_login();
};

async function is_login() {
  let result = await invoke("is_login", {});

  if (result == true) {
    loginForm.hidden = true;
    mainContent.hidden = false;
  } else {
    loginForm.hidden = false;
    mainContent.hidden = true;
  }
}

// 登录处理
loginBtn.addEventListener("click", async (e) => {
  e.preventDefault();

  const username = usernameInput.value.trim();
  const password = passwordInput.value.trim();

  if (!username || !password) {
    loginMessage.textContent = "请输入用户名和密码";
    return;
  }

  try {
    const result = await invoke("check_login", { username, password });
    if (result.success) {
      loginForm.hidden = true;
      mainContent.hidden = false;
      loginMessage.textContent = "";
    } else {
      loginMessage.textContent = result.message;
    }
  } catch (error) {
    loginMessage.textContent = "登录失败，请重试";
    console.error(error);
  }
});

// 连接处理
connectBtn.addEventListener("click", async (e) => {
  e.preventDefault();
  let connect = await invoke("connect", {});
  if (connect == "success") {
    stat.textContent = "已连接";
    stat.classList.add('set-color');
    messagep.textContent = "地址端口为 http（127.0.0.1:9910） socks5（127.0.0.1：9909）";
    connectBtn.disabled = true;
  } else {
    stat.textContent = "未连接";
    messagep.textContent = "连接失败";
  }
});

// 断开处理
disconnectBtn.addEventListener("click", async (e) => {
  e.preventDefault();
  let _ = await invoke("disconnect", {});
  stat.textContent = "未连接";
  stat.classList.remove('set-color');
  messagep.textContent = "地址端口未连接";
  connectBtn.disabled = false;
});
