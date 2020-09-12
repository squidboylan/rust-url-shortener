-- Your SQL goes here
CREATE OR REPLACE FUNCTION make_uid() RETURNS text AS $$
DECLARE
new_uid varchar(8);
done bool;
BEGIN
    done := false;
    WHILE NOT done LOOP
        new_uid := substring(md5(''||now()::text||random()::text) from 1 for 8);
        done := NOT exists(SELECT 1 FROM links WHERE id=new_uid);
END LOOP;
RETURN new_uid;
END;
$$ LANGUAGE PLPGSQL VOLATILE;

CREATE TABLE links (
      id VARCHAR(8) PRIMARY KEY NOT NULL DEFAULT make_uid(),
      dest_url VARCHAR NOT NULL,
      count INT NOT NULL DEFAULT 0
)
