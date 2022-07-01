// use std::process::{Child, ChildStdout, Command, Stdio};

// use serde::{Deserialize, Serialize};

// #[derive(Serialize, Deserialize)]
// struct Endpoints {
//     pub request: Vec<String>,
// }

// pub fn get_endpoints() {
//     let mut child = Command::new("kubectl")
//         .arg("get")
//         .stdin(Stdio::piped())
//         .stdout(Stdio::piped())
//         .spawn()
//         .expect("Error spawning rules process");

//     let stdout = child.stdout.take().unwrap();
// }
