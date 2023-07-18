-- Add migration script here


CREATE TABLE scheduled_scrim_unban (
                user_id bigint PRIMARY KEY,
                expires_at timestamp,
                roles bigint[]
            );
CREATE TABLE user_note (
                user_id bigint,
                id integer,
                created_at timestamp,
                note text,
                creator bigint,
                PRIMARY KEY(user_id, id)
            );
CREATE TABLE reaction (
                user_id bigint PRIMARY KEY,
                emoji text,
                trigger text
            );
CREATE TABLE screenshare (
                user_id bigint PRIMARY KEY,
                creator_id bigint,
                in_question bigint
            );
CREATE TABLE screensharer_stats (
                user_id bigint PRIMARY KEY,
                freezes integer
            );
CREATE TABLE freezes (
                user_id bigint PRIMARY KEY,
                roles bigint[]
            );
