use anyhow::{anyhow, Result};
use aruna_rust_api::api::hooks::services::v2::hook_callback_request::Status;
use aruna_rust_api::api::hooks::services::v2::{Finished, HookCallbackRequest};
use aruna_rust_api::api::hooks::services::v2::hooks_service_client::HooksServiceClient;
use axum::{
    routing::{get, post},
    http::StatusCode,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use aruna_rust_api::api::storage::models::v2::generic_resource::Resource;
use aruna_rust_api::api::storage::models::v2::{KeyValue, KeyValueVariant};
use regex::Regex;


#[tokio::main]
async fn main() -> Result<()> {

    dotenvy::from_filename(".env")?;

    let app = Router::new()
        .route("/", get(root))
        .route("/validate", post(validation));

    let listener = tokio::net::TcpListener::bind(dotenvy::var("SERVER_ADDRESS")?)
        .await
        ?;

    axum::serve::serve(listener, app).await?;

    println!("Running");
    Ok(())
}

async fn validator(request: Request) -> Result<String> {
    if let Some(link) = request.download {

        let file = reqwest::get(link).await?.text().await?;
        let object_id = match request.object {
            Resource::Project(res) => {res.id}
            Resource::Collection(res) => {res.id}
            Resource::Dataset(res) => {res.id}
            Resource::Object(res) => {res.id}
        };

        dbg!(&file);
        let fasta_regex = Regex::new(r"^[^\S\n]*>[^\s>].*(?:\n[^\S\n]*[AGTC]+)+$")?;
        let mut hooks_client = HooksServiceClient::connect(dotenvy::var("ARUNA_ADDRESS")?)
            .await
            .unwrap();
        let response = if fasta_regex.is_match(&file) {
            let request = tonic::Request::new(HookCallbackRequest {
                status: Some(Status::Finished(Finished {
                    add_key_values: vec![KeyValue {
                        key: "FASTA_VALIDATOR".to_string(),
                        value: "successful".to_string(),
                        variant: KeyValueVariant::Label as i32,
                    }],
                    remove_key_values: vec![],
                })),
                secret: request.secret,
                hook_id: request.hook_id,
                object_id,
                pubkey_serial: request.pubkey_serial,
            });

            dbg!(&request);
            let result = hooks_client.hook_callback(request).await?;
            dbg!(result);

            "Is fasta".to_string()
        } else {
            let request = tonic::Request::new(HookCallbackRequest {
                status: Some(Status::Finished(Finished {
                    add_key_values: vec![KeyValue {
                        key: "FASTA_VALIDATOR".to_string(),
                        value: "unsuccessful".to_string(),
                        variant: KeyValueVariant::Label as i32,
                    }],
                    remove_key_values: vec![],
                })),
                secret: request.secret,
                hook_id: request.hook_id,
                object_id,
                pubkey_serial: request.pubkey_serial,
            });

            dbg!(&request);
            let result = hooks_client.hook_callback(request).await?;
            dbg!(result);
            "Is not fasta".to_string()
        };
        Ok(response)
    } else {
        Err(anyhow!("No download url provided"))
    }
}
async fn validation(Json(request): Json<Request>) -> (StatusCode, Json<String>) {
    dbg!(&request);
    
    match validator(request).await {
        Ok(res) => {
            dbg!(&res);
            (StatusCode::OK, Json(res))
        },
        Err(err) => {
            dbg!(&err);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(err.to_string()))
        }
    }
}

async fn root() -> String {
    let id = dotenvy::var("HOOK_ID").unwrap();
    format!("I am a FASTA validation service registered under id {id}")
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Request{
    pub hook_id: String,
    pub object: Resource,
    pub secret: String,
    pub download: Option<String>,
    pub pubkey_serial: i32,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
}