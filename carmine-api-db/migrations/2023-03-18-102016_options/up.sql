CREATE TABLE options (
    id                      serial NOT NULL,
    option_side             SmallInt NOT NULL,
    maturity                Int8 NOT NULL,
    strike_price            Text NOT NULL,
    quote_token_address     Text NOT NULL,
    base_token_address      Text NOT NULL,
    option_type             SmallInt NOT NULL,
    option_address          Text NOT NULL,
    CONSTRAINT options_pkey PRIMARY KEY (id),
    UNIQUE(option_address)
)