use crate::{
    http_clients::OnionClient,
    loki::{LokiServer, ServiceNode},
    onions::NextHop,
};

use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct FileServer {
    pub host: &'static str,
    pub pubkey: &'static str,
}

pub const CHAT_GETSESSION_ORG: FileServer = FileServer {
    host: "chat.getsession.org",
    pubkey: "be12df7bff19f0ab4ed5d14ae5d8d75d91120781929958a553035e51a48a902d",
};

pub const DEV_OPEN_GROUP_SERVER: FileServer = FileServer {
    host: "chat-dev.lokinet.org",
    pubkey: "c5c256d1e1b32f8e20e05b05c47b8ea435b667fb571392db2c6c4f6b1ccf9422",
};

pub const PRODUCTION_FILESERVER: FileServer = FileServer {
    host: "file.getsession.org",
    pubkey: "62509d59bdeec404dd0d489c1e15ba8f94fd3d619b01c1bf48a9922bfcb7311c",
};

pub const DEV_FILESERVER: FileServer = FileServer {
    host: "file-dev.getsession.org",
    pubkey: "2662315c4e728fdbdec61f69eca2316bff267aa8931197907a1c944c7c4e667a",
};

fn parse_file_response_v3(res: &serde_json::Value) -> Result<Vec<u8>, String> {
    let body = res.get("body").ok_or("No body in json")?;

    let data_base64 = body.as_str().ok_or("body is not a string")?;

    let data = base64::decode(data_base64).map_err(|_| "body is not base64 encoded")?;

    Ok(data)
}

pub async fn get_file_via_onion(
    client: &mut OnionClient,
    server: &FileServer,
    token: &str,
    file: &str,
) -> Result<String, &'static str> {
    let endpoint = format!("loki/v1/f/{}", file);

    let auth_header = format!("Bearer {}", token);

    let payload = serde_json::json!({
        "method": "GET",
        "body": "",
        "headers": {"Authorization": auth_header}, // might have Authorization header
        "endpoint": endpoint
    });

    let res = client
        .onion_to_server(server, payload)
        .await
        .map_err(|_| "Could not send")?;

    println!("`res` size: {}", res.len());

    Ok("TODO".to_owned())
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct TokenResponse {
    pub cipherText64: String,
    pub serverPubKey64: String,
}

pub async fn get_token(server: &FileServer) -> Result<String, String> {
    let client = reqwest::Client::new();

    let target = "/loki/v1/get_challenge";
    let url = format!("https://{}{}", server.host, target);

    let (seckey, pubkey) = crate::ecdh::gen_keypair();

    let pubkey_hex = hex::encode(&pubkey);

    let res = client
        .get(&url)
        .query(&[("pubKey", &pubkey_hex)])
        .send()
        .await
        .map_err(|e| format!("Could not send request: {}", e))?;

    let success = res.status().is_success();

    if !success {
        let msg = format!("Status is not OK: {}", res.status());
        return Err(msg);
    }

    let res_body = res
        .text()
        .await
        .map_err(|_| "could not get response body")?;

    let token_res: TokenResponse = serde_json::from_str(&res_body).map_err(|_| "invalid json")?;

    let mut peer_pk = base64::decode(&token_res.serverPubKey64).expect("invalid base64");

    // remove 05
    peer_pk.remove(0);

    let res =
        crate::ecdh::aes_cbc_derive_and_decrypt(token_res.cipherText64.clone(), seckey, &peer_pk)
            .expect("invalid ciphertext");

    submit_token(&pubkey_hex, &res, server).await?;

    Ok(res)
}

async fn submit_token(pubkey: &str, token: &str, server: &FileServer) -> Result<(), String> {
    let target = "/loki/v1/submit_challenge";
    let url = format!("https://{}{}", server.host, target);

    let json = serde_json::json!({
            "pubKey": pubkey, "token": token
    });

    let client = reqwest::Client::new();

    let res = client
        .post(&url)
        .json(&json)
        .send()
        .await
        .map_err(|_| "Could not submit token".to_owned())?;

    if res.status().is_success() {
        return Ok(());
    } else {
        return Err(format!("Non 200 status on token submit: {}", res.status()));
    }
}

pub async fn upload_file_via_onion(client: &mut OnionClient, server: &FileServer) {
    let file = std::fs::read_to_string("image.dat").expect("Could not open file");
    let content_type =
        "multipart/form-data; boundary=--------------------------385203310880548241983752";

    let file_bin = base64::decode(&file).expect("Not valid base64");

    println!("Uploading file len: {}", &file_bin.len());

    let payload_obj = serde_json::json!({
        "method": "POST",
        "body": {"fileUpload": file},
        "headers": {"Authorization": "Bearer loki", "content-type": content_type},
        "endpoint": "files"
    });

    if let Ok(res) = client.onion_to_server(server, payload_obj).await {
        let res: serde_json::Value = serde_json::from_str(&res).expect("Invalid JSON");

        let body = res.get("body").expect("no `body` field");

        let body_str = body.as_str().expect("`body` is not a string");

        let body: serde_json::Value =
            serde_json::from_str(body_str).expect("`body` is not JSON string");

        // println!("body: {}", serde_json::to_string_pretty(&body).unwrap());

        let data = body.get("data").expect("no 'data' field");

        let file_token = data.get("file_token").expect("no 'file_token' field");

        println!("res.body.data.file_token: {}", file_token);
    }
}
