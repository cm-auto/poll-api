create type poll_type as enum ('single', 'multiple');

create table poll
(
    id bigserial primary key,
    title varchar(50) not null,
    poll_type poll_type not null,
    created_at timestamptz not null default now(),
    timeout_at timestamptz not null default now() + '30 minutes',
    delete_at timestamptz not null default now() + '7 days',
    constraint check_timeout_at_higher_than_created_at check (timeout_at > created_at),
    constraint check_delete_at_higher_or_equal_than_timeout_at check (delete_at >= timeout_at)
);

create table poll_option
(
    id bigserial primary key,
    name varchar(50) not null,
    poll_id bigint not null references poll(id) on delete cascade,
    constraint unique_name_poll_id unique (name, poll_id)
);

-- could we make a constraint so that a poll
-- is unique by option_id and ip_address
-- only if the poll has a poll_type of single?
-- or should this just be handled by program logic?
create table poll_vote
(
    id bigserial primary key,
    option_id bigint not null references poll_option(id) on delete cascade,
    ip_address inet not null,
    created_at timestamptz not null default now()
);