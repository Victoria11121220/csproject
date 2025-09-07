pub mod debug;
pub mod iot;
pub mod legacy;
pub mod mas_monitor;

use enum_dispatch::enum_dispatch;
use sea_orm::DatabaseConnection;

#[enum_dispatch]
pub trait StoreReading {
    async fn store(&self, db: &DatabaseConnection, flow_id: i32) -> Result<(), String>;
}

#[derive(Debug, PartialEq)]
#[enum_dispatch(StoreReading)]
pub enum Readings {
    Iot(iot::IoTReading),
    Debug(debug::DebugReading),
    Legacy(legacy::LegacyReading),
    MASMonitor(mas_monitor::MASMonitorReading,),
    Empty(EmptyReading),
}

#[derive(Debug, PartialEq)]
pub struct EmptyReading {}
impl StoreReading for EmptyReading {
    async fn store(&self, _: &DatabaseConnection, _: i32) -> Result<(), String> {
        Ok(())
    }
}