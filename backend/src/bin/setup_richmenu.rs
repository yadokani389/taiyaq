// リッチメニューを上書きします。
// cargo run --bin setup_richmenu

use bot_sdk_line::client::LINE;
use bot_sdk_line::messaging_api_line::{
    apis::{MessagingApiApi, MessagingApiBlobApi},
    models::{
        RichMenuArea, RichMenuBounds, RichMenuRequest, RichMenuSize,
        action::Action as RichMenuAction, postback_action::InputOption,
    },
};
use dotenvy::dotenv;

/// シンプルなリッチメニューを作成（上下分割、下段3等分）
/// =============================
///           通知登録
/// =============================
///  アクセス || メニュー || ヘルプ
/// =============================
fn create_rich_menu_areas() -> Vec<RichMenuArea> {
    vec![
        // 上段: 通知登録 (全幅)
        RichMenuArea {
            bounds: Some(Box::new(RichMenuBounds {
                x: Some(0),
                y: Some(0),
                width: Some(2500),
                height: Some(843),
            })),
            action: Some(Box::new(RichMenuAction::PostbackAction(
                bot_sdk_line::messaging_api_line::models::PostbackAction {
                    r#type: None,
                    label: None,
                    data: Some("action=register_notification".to_string()),
                    display_text: None,
                    text: None,
                    input_option: Some(InputOption::OpenKeyboard),
                    fill_in_text: Some("!adding_notification: ".to_string()),
                },
            ))),
        },
        // 下段左: アクセス (1/3)
        RichMenuArea {
            bounds: Some(Box::new(RichMenuBounds {
                x: Some(0),
                y: Some(843),
                width: Some(833),
                height: Some(843),
            })),
            action: Some(Box::new(RichMenuAction::PostbackAction(
                bot_sdk_line::messaging_api_line::models::PostbackAction {
                    r#type: None,
                    label: None,
                    data: Some("action=show_access".to_string()),
                    display_text: None,
                    text: None,
                    input_option: None,
                    fill_in_text: None,
                },
            ))),
        },
        // 下段中央: メニュー (1/3)
        RichMenuArea {
            bounds: Some(Box::new(RichMenuBounds {
                x: Some(833),
                y: Some(843),
                width: Some(834),
                height: Some(843),
            })),
            action: Some(Box::new(RichMenuAction::PostbackAction(
                bot_sdk_line::messaging_api_line::models::PostbackAction {
                    r#type: None,
                    label: None,
                    data: Some("action=show_menu".to_string()),
                    display_text: None,
                    text: None,
                    input_option: None,
                    fill_in_text: None,
                },
            ))),
        },
        // 下段右: ヘルプ (1/3)
        RichMenuArea {
            bounds: Some(Box::new(RichMenuBounds {
                x: Some(1667),
                y: Some(843),
                width: Some(833),
                height: Some(843),
            })),
            action: Some(Box::new(RichMenuAction::PostbackAction(
                bot_sdk_line::messaging_api_line::models::PostbackAction {
                    r#type: None,
                    label: None,
                    data: Some("action=show_help".to_string()),
                    display_text: None,
                    text: None,
                    input_option: None,
                    fill_in_text: None,
                },
            ))),
        },
    ]
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let channel_access_token = std::env::var("LINE_CHANNEL_ACCESS_TOKEN")
        .map_err(|_| "LINE_CHANNEL_ACCESS_TOKEN not set")?;

    let line = LINE::new(channel_access_token);

    println!("Setting up Rich Menu...\n");

    // 1. 既存のリッチメニューを削除
    println!("Cleaning up old rich menus...");
    let rich_menu_list = line
        .messaging_api_client
        .get_rich_menu_list()
        .await
        .map_err(|e| format!("Failed to get rich menu list: {:?}", e))?;

    for menu in rich_menu_list.richmenus {
        let menu_id = menu.rich_menu_id;
        line.messaging_api_client
            .delete_rich_menu(&menu_id)
            .await
            .map_err(|e| format!("Failed to delete rich menu {}: {:?}", menu_id, e))?;
        println!("✅ Deleted Rich Menu: {}", menu_id);
    }

    // 2. 新しいリッチメニューを作成
    println!("\nCreating new rich menu...");
    let rich_menu_request = RichMenuRequest {
        size: Some(Box::new(RichMenuSize {
            width: Some(2500),
            height: Some(1686),
        })),
        selected: Some(true),
        name: Some("たいやきボット".to_string()),
        chat_bar_text: Some("通知登録はこちらから".to_string()),
        areas: Some(create_rich_menu_areas()),
    };

    let response = line
        .messaging_api_client
        .create_rich_menu(rich_menu_request)
        .await
        .map_err(|e| format!("Failed to create rich menu: {:?}", e))?;

    let rich_menu_id = response.rich_menu_id;

    println!("✅ Created Rich Menu ID: {}", rich_menu_id);

    // 3. 画像をアップロード（2500x1686pxのPNG画像を用意してください）
    println!("\nUploading rich menu image...");
    let image_bytes = std::fs::read("assets/richmenu.png")?;

    // MessagingApiBlobApi の set_rich_menu_image を使用
    line.messaging_api_blob_client
        .set_rich_menu_image(&rich_menu_id, image_bytes)
        .await
        .map_err(|e| format!("Failed to upload rich menu image: {:?}", e))?;

    println!("✅ Uploaded image for Rich Menu: {}", rich_menu_id);

    // 4. デフォルトに設定
    println!("\nSetting as default rich menu...");
    line.messaging_api_client
        .set_default_rich_menu(&rich_menu_id)
        .await
        .map_err(|e| format!("Failed to set default rich menu: {:?}", e))?;

    println!("✅ Set default Rich Menu: {}", rich_menu_id);

    println!("\n✨ Rich Menu setup complete!");
    println!("Rich Menu ID: {}", rich_menu_id);

    Ok(())
}
