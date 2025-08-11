use sea_orm_migration::prelude::*;
use sea_orm::Statement;

// 定义表和列 (枚举类型将在 SQL 中直接处理)
#[derive(DeriveIden)]
enum Site {
    Table,
    Id,
    Name,
    Description,
    Location,
    Public,
    InitialView,
    CreatedAt,
    UpdatedAt,
    ProjectionString,
}

#[derive(DeriveIden)]
enum IotFlow {
    Table,
    Id,
    SiteId,
    Name,
    Nodes,
    Edges,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Sensor {
    Table,
    Id,
    FlowId,
    Identifier,
    Measuring,
    Unit, // We'll handle enum creation in raw SQL
    ValueType, // We'll handle enum creation in raw SQL
    SiteId,
}

#[derive(DeriveIden)]
enum Reading {
    Table,
    Id,
    SensorId,
    Timestamp,
    RawValue,
    Value,
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        
        // 1. 创建枚举类型 (使用 execute_unprepared)
        db.execute_unprepared(r#"CREATE TYPE "iot_field_type" AS ENUM ('BOOLEAN', 'NUMBER', 'STRING');"#).await?;
        db.execute_unprepared(r#"CREATE TYPE "iot_unit" AS ENUM ('AMOUNT', 'AMPERE', 'BOOLEAN', 'CUBIC_METER', 'DEGREES_CELCIUS', 'METER_PER_SECOND', 'PERCENT', 'STRING', 'VOLT', 'WATT', 'WATT_HOUR');"#).await?;
        db.execute_unprepared(r#"CREATE TYPE "iot_source_type" AS ENUM ('HTTP', 'MAS-MONITOR', 'MQTT');"#).await?;

        // 2. 创建 site 表
        manager
            .create_table(
                Table::create()
                    .table(Site::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Site::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(Site::Name).text().not_null().unique_key())
                    .col(ColumnDef::new(Site::Description).text().not_null())
                    .col(ColumnDef::new(Site::Location).json_binary().not_null())
                    .col(ColumnDef::new(Site::Public).boolean().not_null())
                    .col(ColumnDef::new(Site::InitialView).json_binary().not_null())
                    .col(ColumnDef::new(Site::CreatedAt).date_time().not_null())
                    .col(ColumnDef::new(Site::UpdatedAt).date_time().not_null())
                    .col(ColumnDef::new(Site::ProjectionString).text().not_null())
                    .to_owned(),
            )
            .await?;

        // 3. 创建 iot_flow 表
        manager
            .create_table(
                Table::create()
                    .table(IotFlow::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(IotFlow::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(IotFlow::SiteId).integer().not_null())
                    .col(ColumnDef::new(IotFlow::Name).text().not_null().unique_key())
                    .col(ColumnDef::new(IotFlow::Nodes).json_binary().not_null())
                    .col(ColumnDef::new(IotFlow::Edges).json_binary().not_null())
                    .col(ColumnDef::new(IotFlow::CreatedAt).date_time().not_null())
                    .col(ColumnDef::new(IotFlow::UpdatedAt).date_time().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_iot_flow_site_id")
                            .from(IotFlow::Table, IotFlow::SiteId)
                            .to(Site::Table, Site::Id)
                            .on_delete(ForeignKeyAction::NoAction)
                            .on_update(ForeignKeyAction::NoAction)
                    )
                    .to_owned(),
            )
            .await?;

        // 4. 创建 sensor 表 (注意 Unit 和 ValueType 列使用自定义类型)
        manager
            .create_table(
                Table::create()
                    .table(Sensor::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Sensor::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(Sensor::FlowId).integer()) // Nullable
                    .col(ColumnDef::new(Sensor::Identifier).text().not_null())
                    .col(ColumnDef::new(Sensor::Measuring).text().not_null())
                    .col(ColumnDef::new(Sensor::Unit).custom(Alias::new("iot_unit")).not_null())
                    .col(ColumnDef::new(Sensor::ValueType).custom(Alias::new("iot_field_type")).not_null())
                    .col(ColumnDef::new(Sensor::SiteId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_sensor_flow_id")
                            .from(Sensor::Table, Sensor::FlowId)
                            .to(IotFlow::Table, IotFlow::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::NoAction)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_sensor_site_id")
                            .from(Sensor::Table, Sensor::SiteId)
                            .to(Site::Table, Site::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::NoAction)
                    )
                    .to_owned(),
            )
            .await?;

        // 5. 创建 reading 表
        manager
            .create_table(
                Table::create()
                    .table(Reading::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Reading::Id).integer().not_null().auto_increment()) // This is not part of the primary key in the model, but we need a single primary key for the table
                    .col(ColumnDef::new(Reading::SensorId).integer().not_null()) // Part of the composite key in model
                    .col(ColumnDef::new(Reading::Timestamp).date_time().not_null()) // Part of the composite key in model
                    .col(ColumnDef::new(Reading::RawValue).text().not_null())
                    .col(ColumnDef::new(Reading::Value).double()) // Nullable
                    // Note: SeaORM models support composite primary keys, but SeaORM Migration's Table builder
                    // requires a single primary key. The model's #[sea_orm(primary_key)] on multiple columns
                    // is handled by the ORM itself when querying. We define SensorId and Timestamp as NOT NULL.
                    // If strict adherence to the model's composite PK is needed at the DB level, 
                    // we could add a unique constraint or use raw SQL for table creation.
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_reading_sensor_id")
                            .from(Reading::Table, Reading::SensorId)
                            .to(Sensor::Table, Sensor::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::NoAction)
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // 回滚操作：按创建顺序的逆序删除
        
        // 删除表
        manager
            .drop_table(Table::drop().table(Reading::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Sensor::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(IotFlow::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Site::Table).to_owned())
            .await?;

        // 删除枚举类型 (使用 execute_unprepared)
        // 使用 DROP TYPE IF EXISTS to avoid errors if they don't exist
        db.execute_unprepared(r#"DROP TYPE IF EXISTS "iot_field_type";"#).await?;
        db.execute_unprepared(r#"DROP TYPE IF EXISTS "iot_unit";"#).await?;
        db.execute_unprepared(r#"DROP TYPE IF EXISTS "iot_source_type";"#).await?;

        Ok(())
    }
}