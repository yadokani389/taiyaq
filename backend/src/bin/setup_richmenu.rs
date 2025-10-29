// リッチメニューを上書きします。
// cargo run --bin setup_richmenu

use dotenvy::dotenv;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct RichMenu {
    size: Size,
    selected: bool,
    name: String,
    #[serde(rename = "chatBarText")]
    chat_bar_text: String,
    areas: Vec<Area>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Size {
    width: u32,
    height: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct Area {
    bounds: Bounds,
    action: Action,
}

#[derive(Serialize, Deserialize, Debug)]
struct Bounds {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum Action {
    #[serde(rename = "postback")]
    Postback {
        data: String,
        #[serde(rename = "displayText", skip_serializing_if = "Option::is_none")]
        display_text: Option<String>,
        #[serde(rename = "inputOption", skip_serializing_if = "Option::is_none")]
        input_option: Option<String>,
        #[serde(rename = "fillInText", skip_serializing_if = "Option::is_none")]
        fill_in_text: Option<String>,
    },
    #[serde(rename = "message")]
    Message { text: String },
}

/// シンプルなリッチメニューを作成（上下分割、下段3等分）
/// =============================
///           通知登録
/// =============================
///  アクセス || メニュー || ヘルプ
/// =============================
async fn create_simple_rich_menu(
    channel_access_token: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();

    let rich_menu = RichMenu {
        size: Size {
            width: 2500,
            height: 1686,
        },
        selected: true,
        name: "たいやきボット".to_string(),
        chat_bar_text: "メニュー".to_string(),
        areas: vec![
            // 上段: 通知登録 (全幅)
            Area {
                bounds: Bounds {
                    x: 0,
                    y: 0,
                    width: 2500,
                    height: 843,
                },
                action: Action::Postback {
                    data: "action=register_notification".to_string(),
                    display_text: None,
                    input_option: Some("openKeyboard".to_string()),
                    fill_in_text: Some("!adding_notification: ".to_string()),
                },
            },
            // 下段左: アクセス (1/3)
            Area {
                bounds: Bounds {
                    x: 0,
                    y: 843,
                    width: 833,
                    height: 843,
                },
                action: Action::Postback {
                    data: "action=show_access".to_string(),
                    display_text: None,
                    input_option: None,
                    fill_in_text: None,
                },
            },
            // 下段中央: メニュー (1/3)
            Area {
                bounds: Bounds {
                    x: 833,
                    y: 843,
                    width: 834,
                    height: 843,
                },
                action: Action::Postback {
                    data: "action=show_menu".to_string(),
                    display_text: None,
                    input_option: None,
                    fill_in_text: None,
                },
            },
            // 下段右: ヘルプ (1/3)
            Area {
                bounds: Bounds {
                    x: 1667,
                    y: 843,
                    width: 833,
                    height: 843,
                },
                action: Action::Postback {
                    data: "action=show_help".to_string(),
                    display_text: None,
                    input_option: None,
                    fill_in_text: None,
                },
            },
        ],
    };

    let response = client
        .post("https://api.line.me/v2/bot/richmenu")
        .header("Authorization", format!("Bearer {}", channel_access_token))
        .header("Content-Type", "application/json")
        .json(&rich_menu)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("Failed to create rich menu: {}", error_text).into());
    }

    let rich_menu_response: serde_json::Value = response.json().await?;
    let id = rich_menu_response["richMenuId"]
        .as_str()
        .ok_or("richMenuId not found")?
        .to_string();

    println!("✅ Created Rich Menu ID: {}", id);
    Ok(id)
}

/// リッチメニュー一覧を取得
async fn list_rich_menus(
    channel_access_token: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let client = Client::new();

    let response = client
        .get("https://api.line.me/v2/bot/richmenu/list")
        .header("Authorization", format!("Bearer {}", channel_access_token))
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("Failed to list rich menus: {}", error_text).into());
    }

    let list_response: serde_json::Value = response.json().await?;
    let ids: Vec<String> = list_response["richmenus"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|menu| menu["richMenuId"].as_str())
        .map(|s| s.to_string())
        .collect();

    println!("📋 Found {} rich menu(s)", ids.len());
    Ok(ids)
}

/// リッチメニューを削除
async fn delete_rich_menu(
    channel_access_token: &str,
    rich_menu_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let response = client
        .delete(format!(
            "https://api.line.me/v2/bot/richmenu/{}",
            rich_menu_id
        ))
        .header("Authorization", format!("Bearer {}", channel_access_token))
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("Failed to delete rich menu: {}", error_text).into());
    }

    println!("✅ Deleted Rich Menu: {}", rich_menu_id);
    Ok(())
}

/// リッチメニューに画像をアップロード
async fn upload_rich_menu_image(
    channel_access_token: &str,
    rich_menu_id: &str,
    image_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let image_bytes = std::fs::read(image_path)?;

    let response = client
        .post(format!(
            "https://api-data.line.me/v2/bot/richmenu/{}/content",
            rich_menu_id
        ))
        .header("Authorization", format!("Bearer {}", channel_access_token))
        .header("Content-Type", "image/png")
        .body(image_bytes)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("Failed to upload image: {}", error_text).into());
    }

    println!("✅ Uploaded image for Rich Menu: {}", rich_menu_id);
    Ok(())
}

/// リッチメニューを全ユーザーのデフォルトに設定
async fn set_default_rich_menu(
    channel_access_token: &str,
    rich_menu_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let response = client
        .post(format!(
            "https://api.line.me/v2/bot/user/all/richmenu/{}",
            rich_menu_id
        ))
        .header("Authorization", format!("Bearer {}", channel_access_token))
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("Failed to set default rich menu: {}", error_text).into());
    }

    println!("✅ Set default Rich Menu: {}", rich_menu_id);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let channel_access_token =
        std::env::var("LINE_CHANNEL_ACCESS_TOKEN").expect("LINE_CHANNEL_ACCESS_TOKEN not set");

    println!("Setting up Rich Menu...\n");

    // 1. 既存のリッチメニューを削除
    println!("Cleaning up old rich menus...");
    let existing_menus = list_rich_menus(&channel_access_token).await?;
    for menu_id in existing_menus {
        delete_rich_menu(&channel_access_token, &menu_id).await?;
    }

    // 2. 新しいリッチメニューを作成
    println!("\nCreating new rich menu...");
    let rich_menu_id = create_simple_rich_menu(&channel_access_token).await?;

    // 3. 画像をアップロード（2500x1686pxのPNG画像を用意してください）
    println!("\nUploading rich menu image...");
    upload_rich_menu_image(&channel_access_token, &rich_menu_id, "assets/richmenu.png").await?;

    // 4. デフォルトに設定
    println!("\nSetting as default rich menu...");
    set_default_rich_menu(&channel_access_token, &rich_menu_id).await?;

    println!("\nRich Menu setup complete!");
    println!("Rich Menu ID: {}", rich_menu_id);

    Ok(())
}
