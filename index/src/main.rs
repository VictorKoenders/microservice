#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;

use rocket::State;
use rocket_contrib::databases::diesel;
use rocket_contrib::json::Json;
use semver::Version;
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Mutex;

#[database("postgres")]
struct DbConn(diesel::PgConnection);

pub struct IndexState {
    pub active_services: Mutex<HashMap<ServiceName, Service>>,
}

impl Default for IndexState {
    fn default() -> IndexState {
        IndexState {
            active_services: Mutex::new({
                let mut map = HashMap::new();
                let name = ServiceName {
                    name: String::from("database"),
                    version: Version::new(0, 1, 0),
                };
                map.insert(
                    name.clone(),
                    Service {
                        name,
                        address: SocketAddr::V4(SocketAddrV4::new(
                            Ipv4Addr::new(127, 0, 0, 1),
                            1234,
                        )),
                        methods: vec![ServiceMethod {
                            name: String::from("get_user"),
                            args: vec![ServiceMethodArgument {
                                name: String::from("id"),
                                r#type: Type(String::from("u64")),
                            }],
                            returning: Type(String::from("u64")),
                        }],
                    },
                );

                map
            }),
        }
    }
}

#[derive(Serialize, Deserialize, Eq, Hash, PartialEq, Clone)]
pub struct ServiceName {
    pub name: String,
    pub version: Version,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Service {
    pub name: ServiceName,
    pub address: SocketAddr,
    pub methods: Vec<ServiceMethod>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ServiceMethod {
    pub name: String,
    pub args: Vec<ServiceMethodArgument>,
    pub returning: Type,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ServiceMethodArgument {
    pub name: String,
    pub r#type: Type,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Type(pub String);

#[get("/api/list")]
pub fn list_services(state: State<IndexState>) -> Result<Json<Vec<Service>>, String> {
    let services = state
        .active_services
        .lock()
        .map_err(|e| format!("Could not retrieve mutex lock: {:?}", e))?;
    let result = services.values().cloned().collect();
    Ok(Json(result))
}

#[get("/api/service/<name>/<version>")]
pub fn get_service(
    state: State<IndexState>,
    name: String,
    version: String,
) -> Result<Json<Service>, String> {
    let version =
        Version::parse(&version).map_err(|e| format!("Could not parse version: {:?}", e))?;
    let services = state
        .active_services
        .lock()
        .map_err(|e| format!("Could not retrieve mutex lock: {:?}", e))?;
    let service = services
        .get(&ServiceName { name, version })
        .ok_or_else(|| String::from("Service not found"))?;
    Ok(Json(service.clone()))
}

fn main() {
    rocket::ignite()
        .manage(IndexState::default())
        .attach(DbConn::fairing())
        .mount("/", routes![list_services, get_service])
        .launch();
}
