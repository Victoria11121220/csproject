package model

import (
	"time"

	"github.com/jmoiron/sqlx"
)

type IotFlow struct {
	ID        int64     `db:"id" json:"id"`
	SiteID    int64     `db:"site_id" json:"site_id"`
	Name      string    `db:"name" json:"name"`
	Nodes     string    `db:"nodes" json:"nodes"`
	Edges     string    `db:"edges" json:"edges"`
	CreatedAt time.Time `db:"created_at" json:"created_at"`
	UpdatedAt time.Time `db:"updated_at" json:"updated_at"`
}

type Database struct {
	*sqlx.DB
}