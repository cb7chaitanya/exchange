-- Your SQL goes here

CREATE TABLE trades (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    is_buyer_maker BOOLEAN NOT NULL,
    price VARCHAR NOT NULL,
    quantity VARCHAR NOT NULL,
    quote_quantity VARCHAR NOT NULL,
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    market VARCHAR NOT NULL
);

CREATE TABLE orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    executed_qty DECIMAL NOT NULL,
    market VARCHAR NOT NULL,
    price VARCHAR NOT NULL,
    quantity VARCHAR NOT NULL,
    side VARCHAR NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
