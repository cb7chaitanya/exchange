// @generated automatically by Diesel CLI.

diesel::table! {
    _sqlx_migrations (version) {
        version -> Int8,
        description -> Text,
        installed_on -> Timestamptz,
        success -> Bool,
        checksum -> Bytea,
        execution_time -> Int8,
    }
}

diesel::table! {
    orders (id) {
        id -> Uuid,
        executed_qty -> Numeric,
        market -> Varchar,
        price -> Varchar,
        quantity -> Varchar,
        side -> Varchar,
        created_at -> Timestamp,
    }
}

diesel::table! {
    trades (id) {
        id -> Uuid,
        is_buyer_maker -> Bool,
        price -> Varchar,
        quantity -> Varchar,
        quote_quantity -> Varchar,
        timestamp -> Timestamp,
        market -> Varchar,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        username -> Varchar,
        email -> Varchar,
        password_hash -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    orders,
    trades,
    users,
);
