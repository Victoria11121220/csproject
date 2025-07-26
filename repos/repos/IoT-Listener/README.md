# IoT-listener
## To generate entities from the database
Set environment variable `DATABASE_URL` and then run:
`sea-orm-cli generate entity -o src/entities --enum-extra-attributes 'serde(rename_all="SCREAMING_SNAKE_CASE")' --with-serde both --ignore-tables seaql_migrations,area,databasechangelog,databasechangeloglock,infohub_page,infohub_page_ownership,iot_deployment,navigation_edge,navigation_point,poi,poi_group,poi_sensor_link,point_cloud`