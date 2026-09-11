# Doopack Rust SDK

O **Doopack Rust SDK** é a biblioteca oficial e extremamente leve para desenvolvimento de microsserviços (Event-Driven) dentro do ecossistema Doopack.

Devido à arquitetura avançada do Doopack, este SDK não precisa gerenciar conexões pesadas de banco de dados nativamente. Ele age comunicando-se com o **Orquestrador Doopack** via variáveis de ambiente, `stdout`, e requisições HTTP Stateless (Database Proxy). Isso garante um **Cold Start (tempo de inicialização) quase zero**.

---

## 📦 Instalação

Se você publicou o SDK no GitHub, basta adicionar ao seu `Cargo.toml`:

```toml
[dependencies]
doopack-rust-sdk = { git = "https://github.com/SEU_USUARIO/doopack-rust-sdk.git" }
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
```

---

## ⚙️ Variáveis de Ambiente

O SDK depende de variáveis de ambiente injetadas **automaticamente** pelo Orquestrador do Doopack no momento em que o microsserviço é executado. 
Para testar localmente na sua máquina, você deve simular a injeção dessas variáveis no seu terminal.

| Variável | Descrição | Exemplo para Teste Local |
|----------|-----------|--------------------------|
| `PAYLOAD_INPUT` | Contém o JSON exato do evento/trigger que acionou o microsserviço. | `PAYLOAD_INPUT='{"user_id": 123}' cargo run` |
| `DOOPACK_PROXY_URL` | URL do Proxy Interno do Orquestrador responsável por executar as queries no banco. | `DOOPACK_PROXY_URL='http://localhost:4500/api/v1/internal/proxy/db/query'` |

*(Nota: Antigamente o SDK lia `DB_POOL_*` para abrir conexões diretas, mas graças à arquitetura de Database Proxy, isso agora é centralizado no Orquestrador para poupar memória do seu microsserviço!)*

---

## 📚 Como fazer um CRUD (Exemplo Completo)

Com o novo SDK, realizar um CRUD no SurrealDB é incrivelmente simples usando o método `query_surreal`. Você não precisa instanciar clientes de banco de dados, o SDK repassa o SQL para o Orquestrador através do Proxy.

O método `query_surreal` recebe:
1. O **nome do Pool** de banco de dados configurado no painel do Doopack (ex: `"MEUBANCO"`).
2. A **query SQL** (no dialeto do SurrealQL).
3. Opcionalmente, um objeto **JSON de bindings** (variáveis) para evitar SQL Injection.

Abaixo, um exemplo completo de um microsserviço que recebe uma ação via payload e realiza um CRUD:

```rust
use doopack_rust_sdk::{get_input, send_output, query_surreal};
use serde_json::json;

#[tokio::main]
async fn main() {
    // 1. Pega os dados que o Doopack injetou na inicialização
    let payload = match get_input() {
        Ok(val) => val,
        Err(_) => json!({ "action": "create", "name": "Doopack Dev" }) // Mock para teste local
    };

    let action = payload["action"].as_str().unwrap_or("read");
    
    // Nome do Pool configurado lá no dashboard do Doopack
    let pool_name = "MEUBANCO";

    // Variável para guardar o resultado do banco
    let mut db_result = json!([]);

    match action {
        // ==========================================
        // C - CREATE (Criar)
        // ==========================================
        "create" => {
            let name = payload["name"].as_str().unwrap_or("Desconhecido");
            
            // Usamos bindings (variáveis $name) para segurança
            let bindings = json!({ "name": name, "status": "active" });
            
            let res = query_surreal(
                pool_name, 
                "CREATE users SET name = $name, status = $status;", 
                Some(bindings)
            ).await;
            
            db_result = res.unwrap_or_default().into();
        }

        // ==========================================
        // R - READ (Ler)
        // ==========================================
        "read" => {
            // Leitura simples, sem bindings
            let res = query_surreal(
                pool_name, 
                "SELECT * FROM users WHERE status = 'active';", 
                None
            ).await;
            
            db_result = res.unwrap_or_default().into();
        }

        // ==========================================
        // U - UPDATE (Atualizar)
        // ==========================================
        "update" => {
            let id_to_update = payload["id"].as_str().unwrap_or("users:123");
            
            let bindings = json!({ "target_id": id_to_update, "new_status": "premium" });
            
            let res = query_surreal(
                pool_name, 
                "UPDATE type::thing($target_id) SET status = $new_status;", 
                Some(bindings)
            ).await;

            db_result = res.unwrap_or_default().into();
        }

        // ==========================================
        // D - DELETE (Deletar)
        // ==========================================
        "delete" => {
            let id_to_delete = payload["id"].as_str().unwrap_or("users:123");
            
            let bindings = json!({ "target_id": id_to_delete });
            
            let res = query_surreal(
                pool_name, 
                "DELETE type::thing($target_id);", 
                Some(bindings)
            ).await;

            db_result = res.unwrap_or_default().into();
        }
        
        _ => {}
    }

    // 2. Imprime a resposta final usando o SDK. 
    // O Doopack orquestrador lê isso e considera o evento como processado com sucesso!
    send_output(&json!({
        "status": "success",
        "action_executed": action,
        "database_response": db_result
    }));
}
```

### Como testar o CRUD localmente:
Para testar na sua máquina, lembre-se de rodar seu backend do Doopack em paralelo (`cargo run -p server` na pasta principal do Doopack) para que o Orquestrador esteja ouvindo os pedidos do Proxy. Depois, no terminal do seu microsserviço:

```bash
# Executar Ação de Ler
PAYLOAD_INPUT='{"action": "read"}' \
DOOPACK_PROXY_URL='http://localhost:4500/api/v1/internal/proxy/db/query' \
cargo run

# Executar Ação de Criar
PAYLOAD_INPUT='{"action": "create", "name": "Weliton"}' \
DOOPACK_PROXY_URL='http://localhost:4500/api/v1/internal/proxy/db/query' \
cargo run
```

E se o microsserviço estiver sendo executado oficialmente na nuvem pelo Doopack, você não precisa passar nada! O orquestrador injetará o `PAYLOAD_INPUT` e a URL correta do Proxy automaticamente!
