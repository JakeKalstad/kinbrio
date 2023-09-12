CREATE TABLE IF NOT EXISTS organization (
    key uuid,
    external_accounting_id text,
    external_accounting_url text,
    owner_key uuid,
    domain varchar(2048),
    contact_email varchar(320),
    name varchar(256),
    description varchar(512),
    matrix_home_server text,
    matrix_live_support_room_url text,
    matrix_general_room_url text,
    created bigint,
    updated bigint
);

CREATE TABLE IF NOT EXISTS users (
    key uuid,
    organization_key uuid,
    email varchar(320),
    matrix_user_id varchar(512),
    matrix_home_server text,
    created bigint,
    updated bigint
);

CREATE TABLE IF NOT EXISTS files (
    key uuid,
    owner_key uuid,
    organization_key uuid,
    association_type smallint,
    association_key uuid,
    url varchar(2048),
    hash text,
    name varchar(256),
    description varchar(512),
    tags varchar(256),
    format varchar(32),
    size bigint,
    created bigint,
    updated bigint
);

CREATE TABLE IF NOT EXISTS notes (
    key uuid,
    owner_key uuid,
    organization_key uuid,
    association_type smallint,
    association_key uuid,
    url varchar(2048),
    hash text,
    title varchar(256),
    content varchar(512),
    created bigint,
    updated bigint
);

CREATE TABLE IF NOT EXISTS service_items (
    key uuid,
    owner_key uuid,
    organization_key uuid,
    external_accounting_id text,
    name varchar(256),
    description varchar(512),
    value bigint,
    currency varchar(16),
    service_item_type smallint,
    service_value_type smallint,
    expenses uuid [],
    created bigint,
    updated bigint
);

CREATE TABLE IF NOT EXISTS projects (
    key uuid,
    owner_key uuid,
    organization_key uuid,
    name varchar(256),
    description varchar(512),
    tags varchar(256),
    estimated_quarter_days int,
    start bigint,
    due bigint,
    created bigint,
    updated bigint
);

CREATE TABLE IF NOT EXISTS mile_stones (
    key uuid,
    owner_key uuid,
    organization_key uuid,
    project_key uuid,
    name varchar(256),
    description varchar(512),
    tags varchar(256),
    estimated_quarter_days int,
    start bigint,
    due bigint,
    created bigint,
    updated bigint
);

CREATE TABLE IF NOT EXISTS tasks (
    key uuid,
    owner_key uuid,
    organization_key uuid,
    project_key uuid,
    assignee_key uuid,
    name varchar(256),
    description varchar(512),
    tags varchar(256),
    status smallint,
    estimated_quarter_days int,
    start bigint,
    due bigint,
    created bigint,
    updated bigint
);

CREATE TABLE IF NOT EXISTS boards (
    key uuid,
    owner_key uuid,
    organization_key uuid,
    project_key uuid,
    name varchar(256),
    description varchar(512),
    columns text [],
    lanes text [],
    filter text,
    created bigint,
    updated bigint
);

CREATE TABLE IF NOT EXISTS entitys (
    key uuid,
    owner_key uuid,
    organization_key uuid,
    external_accounting_id text,
    name varchar(256),
    description varchar(512),
    matrix_room_url text,
    web_url text,
    avatar_url text,
    entity_type smallint,
    address_primary text,
    address_unit text,
    city text,
    state text,
    zip_code text,
    country text,
    created bigint,
    updated bigint
);

CREATE TABLE IF NOT EXISTS contacts (
    key uuid,
    entity_key uuid,
    organization_key uuid,
    external_accounting_id text,
    first_name varchar(128),
    middle_initial varchar(12),
    last_name varchar(128),
    description varchar(512),
    position varchar(512),
    email varchar(320),
    phone varchar(16),
    secondary_email varchar(320),
    secondary_phone varchar(16),
    matrix_user_id varchar(512),
    web_url text,
    avatar_url text,
    social_urls text [],
    address_primary text,
    address_unit text,
    city text,
    state text,
    zip_code text,
    country text,
    created bigint,
    updated bigint
);

CREATE TABLE IF NOT EXISTS rooms (
    key uuid,
    owner_key uuid,
    organization_key uuid,
    name varchar(256),
    description varchar(512),
    matrix_room_url text,
    matrix_room_id varchar(512),
    message_types smallint,
    alert_level smallint,
    created bigint,
    updated bigint
);

CREATE TABLE IF NOT EXISTS akaunting_options (
    key uuid,
    owner_key uuid,
    organization_key uuid,

    matrix_room_url text,
    user_name text,
    user_pass text,
    akaunting_domain text,
    akaunting_company_id text,

    organization_data boolean,
    employee_data boolean,
    client_data boolean,
    vendor_data boolean,
    item_data boolean,
    invoice_data boolean,
    allow_post boolean,
    last_sync bigint,
    created bigint,
    updated bigint
);


GRANT ALL ON ALL TABLES IN SCHEMA public TO cloudify2;

delete from akaunting_options;
delete from boards           ;
delete from contacts         ;
delete from entitys          ;
delete from files            ;
delete from mile_stones      ;
delete from notes            ;
delete from organization     ;
delete from projects         ;
delete from rooms            ;
delete from service_items    ;
delete from tasks            ;
delete from users;