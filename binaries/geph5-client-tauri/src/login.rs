use geph5_broker_protocol::{BrokerClient, Credential};
// use poll_promise::Promise;
use serde::Serialize;

use crate::settings::{PASSWORD, USERNAME, get_config};

// pub struct Login {
//     username: String,
//     password: String,

//     check_login: Option<Promise<Result<LoginResponse, String>>>,
// }

// 添加 Serialize 以支持返回值序列化
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    success: bool,
    message: Option<String>,
}

// impl Default for Login {
//     fn default() -> Self {
//         Self::new()
//     }
// }

// impl Login {
// pub fn new() -> Self {
//     Self {
//         username: "".to_string(),
//         password: "".to_string(),

//         check_login: None,
//     }
// }

// pub fn is_login(&self) -> bool {
//     self.check_login.is_some()
// }

// fn check_login(&mut self) -> Result<LoginResponse, String> {
//     if let Some(promise) = self.check_login.as_ref() {
//         match promise.poll() {
//             std::task::Poll::Ready(ready) => match ready {
//                 Ok(_) => {
//                     self.check_login = None;
//                     USERNAME.set(self.username.clone());
//                     PASSWORD.set(self.password.clone());
//                     Ok(LoginResponse {
//                         success: true,
//                         message: None,
//                     })
//                 }
//                 Err(err) => {
//                     let err = format!("{:?}", err);
//                     return Err(err);
//                 }
//             },
//             std::task::Poll::Pending => {
//                 return Err("Pending".to_owned());
//             }
//         }
//     } else {
//         let username = self.username.clone();
//         let password = self.password.clone();
//         self.check_login = Some(Promise::spawn_thread("check_login", move || {
//             smolscale::block_on(check_login(username, password))
//         }));

//         return Err("Not login".to_owned());
//     }
// }

//     pub fn render(&mut self, ui: &mut egui::Ui) -> anyhow::Result<()> {
//         if let Some(promise) = self.check_login.as_ref() {
//             ui.add_space(30.0);
//             match promise.poll() {
//                 std::task::Poll::Ready(ready) => match ready {
//                     Ok(_) => {
//                         self.check_login = None;
//                         USERNAME.set(self.username.take());
//                         PASSWORD.set(self.password.take());
//                     }
//                     Err(err) => {
//                         let err = format!("{:?}", err);
//                         ui.vertical_centered(|ui| {
//                             ui.colored_label(egui::Color32::DARK_RED, err);
//                             if ui.button(l10n("ok")).clicked() {
//                                 self.check_login = None;
//                             }
//                         });
//                     }
//                 },
//                 std::task::Poll::Pending => {
//                     ui.vertical_centered(|ui| {
//                         ui.label(l10n("logging_in"));
//                         ui.spinner();
//                     });
//                 }
//             }
//         } else {
//             let (rect, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::click());
//             let rect = rect.shrink2(egui::vec2(40., 0.));
//             ui.allocate_ui_at_rect(rect, |ui| {
//                 ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
//                     ui.add_space(10.);
//                     Image::new(egui::include_image!("../../icon.png"))
//                         .fit_to_exact_size(egui::vec2(140., 140.))
//                         .ui(ui);
//                     ui.add_space(10.);
//                     let username_edit = TextEdit::singleline(&mut self.username)
//                         .hint_text(l10n("username"))
//                         .ui(ui);
//                     let password_edit = TextEdit::singleline(&mut self.password)
//                         .hint_text(l10n("password"))
//                         .password(true)
//                         .ui(ui);
//                     if username_edit.clicked() || password_edit.clicked() {
//                         show_keyboard(true)
//                     }
//                     if username_edit.clicked_elsewhere() && password_edit.clicked_elsewhere() {
//                         show_keyboard(false)
//                     }
//                     anyhow::Ok(())
//                 })
//                 .inner?;

//                 if ui.button(l10n("login")).clicked() || ui.input(|i| i.key_pressed(Key::Enter)) {
//                     let username = self.username.clone();
//                     let password = self.password.clone();
//                     self.check_login = Some(Promise::spawn_thread("check_login", move || {
//                         smolscale::block_on(check_login(username, password))
//                     }));
//                 }
//                 anyhow::Ok(())
//             })
//             .inner?;
//         }

//         Ok(())
//     }
// }

#[tauri::command]
pub async fn check_login(username: String, password: String) -> Result<LoginResponse, String> {
    let mut config = match get_config() {
        Ok(config) => config,
        Err(e) => {
            return Ok(LoginResponse {
                success: false,
                message: Some(format!("Failed to get config: {}", e)),
            });
        }
    };

    let user: String = username.clone();
    let pass: String = password.clone();

    config.credentials = Credential::LegacyUsernamePassword { username, password };

    match config.broker {
        Some(broker) => {
            let rpc_transport = broker.rpc_transport();
            let client = BrokerClient::from(rpc_transport);

            match client.get_auth_token(config.credentials.clone()).await {
                Ok(Ok(_)) => {
                    USERNAME.set(user);
                    PASSWORD.set(pass);
                    Ok(LoginResponse {
                        success: true,
                        message: None,
                    })
                }
                _ => Ok(LoginResponse {
                    success: false,
                    message: Some(format!("Error: {:?}", "Invalid username or password")),
                }),
            }
        }
        None => Ok(LoginResponse {
            success: false,
            message: Some("No broker configured".to_string()),
        }),
    }
}
