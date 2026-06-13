use taiyaq_backend::api::openapi::build_openapi;

fn main() -> anyhow::Result<()> {
    let api_doc = build_openapi();
    println!("{}", serde_json::to_string_pretty(&api_doc)?);
    Ok(())
}
