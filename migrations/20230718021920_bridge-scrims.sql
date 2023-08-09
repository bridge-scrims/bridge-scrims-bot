-- Add migration script here


CREATE TABLE scheduled_scrim_unban (
                user_id bigint PRIMARY KEY,
                expires_at TIMESTAMP WITH TIME ZONE,
                roles bigint[] NOT NULL
            );
CREATE TABLE user_note (
                user_id BIGINT,
                id INTEGER,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL,
                note TEXT NOT NULL,
                creator BIGINT NOT NULL,
                PRIMARY KEY(user_id, id)
            );
CREATE TABLE reaction (
                user_id BIGINT PRIMARY KEY,
                emoji TEXT NOT NULL,
                trigger TEXT NOT NULL
            );
CREATE TABLE screenshare (
                channel_id BIGINT PRIMARY KEY,
                creator_id BIGINT NOT NULL,
                in_question BIGINT NOT NULL
            );
CREATE TABLE screensharer_stats (
                user_id BIGINT PRIMARY KEY,
                freezes INTEGER NOT NULL
            );
CREATE TABLE freezes (
                user_id BIGINT PRIMARY KEY,
                roles BIGINT[] NOT NULL
            );
