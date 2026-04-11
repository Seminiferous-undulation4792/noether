use noether_engine::executor::composite::CompositeExecutor;
use noether_engine::executor::runner::run_composition;
use noether_engine::lagrange::parse_graph;
use noether_store::StageStore;
use std::io::{Read, Write};
use std::net::TcpListener;

pub fn cmd_serve(
    store: &dyn StageStore,
    executor: &CompositeExecutor,
    graph_path: &str,
    bind: &str,
) {
    // Parse the graph once at startup
    let content = match std::fs::read_to_string(graph_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read {graph_path}: {e}");
            std::process::exit(1);
        }
    };
    let graph = match parse_graph(&content) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Invalid graph JSON: {e}");
            std::process::exit(1);
        }
    };

    // Type check before serving
    if let Err(errors) = noether_engine::checker::check_graph(&graph.root, store) {
        let msgs: Vec<String> = errors.iter().map(|e| format!("{e}")).collect();
        eprintln!("Graph type check failed:\n  {}", msgs.join("\n  "));
        std::process::exit(1);
    }

    let addr = if bind.starts_with(':') {
        format!("0.0.0.0{bind}")
    } else {
        bind.to_string()
    };

    let listener = TcpListener::bind(&addr).unwrap_or_else(|e| {
        eprintln!("Cannot bind to {addr}: {e}");
        std::process::exit(1);
    });

    eprintln!("noether serve: {}", graph.description);
    eprintln!("Listening on http://{addr}");
    eprintln!("  POST /       — run the graph with JSON body as input");
    eprintln!("  GET  /health  — health check");
    eprintln!("  Press Ctrl+C to stop");

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };

        let mut buf = [0u8; 65536];
        let n = stream.read(&mut buf).unwrap_or(0);
        let request = String::from_utf8_lossy(&buf[..n]);

        // Parse HTTP request line
        let first_line = request.lines().next().unwrap_or("");
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        let (method, path) = if parts.len() >= 2 {
            (parts[0], parts[1])
        } else {
            ("GET", "/")
        };

        // Extract body (after \r\n\r\n)
        let body = request
            .split("\r\n\r\n")
            .nth(1)
            .unwrap_or("")
            .trim_end_matches('\0')
            .to_string();

        let (status, response_body) = match (method, path) {
            ("GET", "/health") => {
                let health = serde_json::json!({
                    "ok": true,
                    "graph": graph.description,
                });
                ("200 OK", serde_json::to_string(&health).unwrap())
            }
            ("POST", "/" | "/run") => {
                let input: serde_json::Value =
                    serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);

                match run_composition(&graph.root, &input, executor, "serve") {
                    Ok(result) => {
                        let resp = serde_json::json!({
                            "ok": true,
                            "output": result.output,
                            "duration_ms": result.trace.duration_ms,
                        });
                        ("200 OK", serde_json::to_string(&resp).unwrap())
                    }
                    Err(e) => {
                        let resp = serde_json::json!({
                            "ok": false,
                            "error": format!("{e}"),
                        });
                        (
                            "500 Internal Server Error",
                            serde_json::to_string(&resp).unwrap(),
                        )
                    }
                }
            }
            _ => {
                let resp = serde_json::json!({"ok": false, "error": "Use POST / with JSON body"});
                ("404 Not Found", serde_json::to_string(&resp).unwrap())
            }
        };

        let http_response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        let _ = stream.write_all(http_response.as_bytes());

        // Log
        eprintln!("{method} {path} → {status}");
    }
}
