CREATE TABLE numerics (
    a BIT,
    b BIT(2),
    c TINYINT,
    d TINYINT(10),
    e BOOL,
    f BOOLEAN,
    g SMALLINT,
    h SMALLINT(100),
    i INT,
    j INT(64),
    k INTEGER,
    l INTEGER(64),
    m BIGINT,
    n BIGINT(100),
    o DECIMAL,
    p DECIMAL(10),
    q DECIMAL(10,2),
    r DEC,
    s DEC(10),
    t DEC(10, 2),
    u FLOAT,
    v FLOAT(10),
    w FLOAT(10,2),
    x DOUBLE,
    y DOUBLE(10,2),
    z DOUBLE,
    aa DOUBLE PRECISION (10, 2)
);

INSERT INTO numerics (a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u, v, w, x, y, z, aa)
    VALUES (1, 2, 10, 123, true, false, 1234, 1234, 12345, 12345, 12345, 12345, 123456, 123456, 10.12345, 10000.12, 10000.12,
       10.12345, 10000.12, 10000.12, 123.456, 123.456, 123.45, 12345.6789, 12345.67, 12345.6789, 12345.67);

SELECT a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u, v, w, x, y, z, aa FROM numerics;

CREATE TABLE characters (
    a NATIONAL CHAR,
    b CHAR,
    c CHAR(255),
    d NCHAR,
    e NCHAR(255),
    f NATIONAL CHARACTER,
    g CHARACTER,
    h CHARACTER(255),
    i NATIONAL CHARACTER(50),
    j VARCHAR(50),
    k NVARCHAR(50),
    l CHARACTER VARYING(50)
);

INSERT INTO characters (a, b, c, d, e, f, g, h, i, j, k, l)
    VALUES('a', 'a', 'chars', 'b', 'nchars', 'b', 'b', 'characters', 'ineedtwentyfivecharacters', 'variableness',
        'nvariableness', 'varying characters');

SELECT a, b, c, d, e, f, g, h, i, j, k, l FROM characters;