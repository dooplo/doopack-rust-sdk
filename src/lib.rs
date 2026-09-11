//! # Doopack Rust SDK
//! 
//! O **Doopack Rust SDK** é a biblioteca oficial para desenvolvimento de microsserviços (Event-Driven) dentro do ecossistema Doopack.
//! Ele utiliza a arquitetura de **Database Proxy**, repassando a carga de conexão pesada para o Orquestrador e garantindo 
//! um Cold Start próximo de zero.
//! 
//! ## Variáveis de Ambiente Injetadas
//! O Doopack Orquestrador injeta variáveis de ambiente cruciais no momento da execução:
//! - `PAYLOAD_INPUT`: O JSON do evento/trigger que acionou o serviço.
//! - `DOOPACK_PROXY_URL`: A URL da API interna do Orquestrador que processará as queries de banco.
//! 
//! ## Exemplo de CRUD com SurrealDB
//! ```rust,no_run
//! use doopack_rust_sdk::{get_input, send_output, query_surreal};
//! use serde_json::json;
//! 
//! #[tokio::main]
//! async fn main() {
//!     // Lendo o input (ex: {"action": "create", "name": "Doopack"})
//!     let payload = get_input().unwrap_or(json!({ "action": "read" }));
//!     let action = payload["action"].as_str().unwrap_or("read");
//!     let pool_name = "MEUBANCO"; // Configurado no painel do Doopack
//! 
//!     let db_result = match action {
//!         "create" => {
//!             let name = payload["name"].as_str().unwrap_or("User");
//!             query_surreal(pool_name, "CREATE users SET name = $name;", Some(json!({ "name": name }))).await
//!         },
//!         "read" => {
//!             query_surreal(pool_name, "SELECT * FROM users;", None).await
//!         },
//!         "update" => {
//!             query_surreal(pool_name, "UPDATE type::thing($id) SET updated = true;", Some(json!({ "id": "users:123" }))).await
//!         },
//!         "delete" => {
//!             query_surreal(pool_name, "DELETE type::thing($id);", Some(json!({ "id": "users:123" }))).await
//!         },
//!         _ => Ok(vec![]),
//!     };
//! 
//!     // Responde ao orquestrador
//!     send_output(&json!({
//!         "status": "success",
//!         "db_response": db_result.unwrap_or_default()
//!     }));
//! }
//! ```
//! 

use std::env;
use serde::Serialize;
use serde_json::Value;

/// Retrieves the input payload passed to the microservice by the orchestrator.
/// This reads the `PAYLOAD_INPUT` environment variable and parses it as JSON.
pub fn get_input() -> Result<Value, String> {
    let input_str = env::var("PAYLOAD_INPUT")
        .map_err(|e| format!("PAYLOAD_INPUT environment variable not found: {}", e))?;
    serde_json::from_str(&input_str)
        .map_err(|e| format!("Failed to parse PAYLOAD_INPUT JSON: {}", e))
}

/// Sends the output of the microservice execution back to the orchestrator.
/// This prints the serialized output to stdout, which is captured by the orchestrator.
pub fn send_output<T: Serialize>(output: &T) {
    if let Ok(json_str) = serde_json::to_string(output) {
        println!("{}", json_str);
    } else {
        println!("{{ \"error\": \"Failed to serialize output\" }}");
    }
}

/// Query a SurrealDB instance configured in Doopack.
/// This sends the query over HTTP to the Doopack Orchestrator Proxy, protecting your database
/// from connection exhaustion.
pub async fn query_surreal(pool_name: &str, query: &str, bindings: Option<Value>) -> Result<Vec<Value>, String> {
    let proxy_url = env::var("DOOPACK_PROXY_URL").unwrap_or_else(|_| "http://localhost:4500/api/v1/internal/proxy/db/query".to_string());
    
    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "pool": pool_name,
        "query": query,
        "bindings": bindings
    });

    let res = client.post(&proxy_url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Proxy request failed: {}", e))?;

    if res.status().is_success() {
        let results: Vec<Value> = res.json().await.map_err(|e| format!("Failed to parse proxy JSON: {}", e))?;
        Ok(results)
    } else {
        let err_text = res.text().await.unwrap_or_default();
        Err(format!("Proxy returned error: {}", err_text))
    }
}
