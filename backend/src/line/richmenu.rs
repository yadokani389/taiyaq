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
    },
    #[serde(rename = "message")]
    Message { text: String },
}

/// シンプルなリッチメニューを作成（2x2レイアウト）
pub async fn create_simple_rich_menu(
    channel_access_token: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();

    // 2x2レイアウト（左上・右上・左下・右下）
    let rich_menu = RichMenu {
        size: Size {
            width: 2500,
            height: 1686,
        },
        selected: true,
        name: "たいやきボット".to_string(),
        chat_bar_text: "メニュー".to_string(),
        areas: vec![
            // 左上: メニュー表示
            Area {
                bounds: Bounds {
                    x: 0,
                    y: 0,
                    width: 1250,
                    height: 843,
                },
                action: Action::Postback {
                    data: "action=show_menu".to_string(),
                    display_text: Some("メニューを表示".to_string()),
                },
            },
            // 右上: 通知登録
            Area {
                bounds: Bounds {
                    x: 1250,
                    y: 0,
                    width: 1250,
                    height: 843,
                },
                action: Action::Message {
                    text: "!notification".to_string(),
                },
            },
            // 左下: アクセス情報
            Area {
                bounds: Bounds {
                    x: 0,
                    y: 843,
                    width: 1250,
                    height: 843,
                },
                action: Action::Postback {
                    data: "action=show_access".to_string(),
                    display_text: Some("アクセス情報を表示".to_string()),
                },
            },
            // 右下: ヘルプ
            Area {
                bounds: Bounds {
                    x: 1250,
                    y: 843,
                    width: 1250,
                    height: 843,
                },
                action: Action::Postback {
                    data: "action=show_help".to_string(),
                    display_text: Some("ヘルプを表示".to_string()),
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

/// リッチメニューに画像をアップロード
pub async fn upload_rich_menu_image(
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
pub async fn set_default_rich_menu(
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

/// 既存のリッチメニューを削除
pub async fn delete_rich_menu(
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

/// リッチメニュー一覧を取得
pub async fn list_rich_menus(
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
