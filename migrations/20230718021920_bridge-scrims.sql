-- Add migration script here


CREATE TABLE scheduled_scrim_unban (
                user_id bigint PRIMARY KEY,
                expires_at timestamp,
                roles bigint[] NOT NULL
            );
CREATE TABLE user_note (
                user_id bigint,
                id integer,
                created_at timestamp NOT NULL,
                note text NOT NULL,
                creator bigint NOT NULL,
                PRIMARY KEY(user_id, id)
            );
CREATE TABLE reaction (
                user_id bigint PRIMARY KEY,
                emoji text NOT NULL,
                trigger text NOT NULL
            );
CREATE TABLE screenshare (
                channel_id bigint PRIMARY KEY,
                creator_id bigint NOT NULL,
                in_question bigint NOT NULL
            );
CREATE TABLE screensharer_stats (
                user_id bigint PRIMARY KEY,
                freezes integer NOT NULL
            );
CREATE TABLE freezes (
                user_id bigint PRIMARY KEY,
                roles bigint[] NOT NULL
            );
