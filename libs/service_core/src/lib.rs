#[macro_use]
extern crate service_core_derive;

#[allow(non_camel_case_types)]
#[mock_service(name = "database", version = "0.1.0")]
pub struct database {}
