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
       10.12345, 10000.12, -10000.12, 123.456, 123.456, 123.45, 12345.6789, 12345.67, 12345.6789, -12345.67);

SELECT a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u, v, w, x, y, z, aa FROM numerics;

SELECT a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u, v, w, x, y, z, aa FROM numerics
    WHERE a = 1 AND b = 2 AND c = 10 AND d = 123 AND e = true AND f = false AND g = 1234 AND h = 1234
     AND i = 12345 AND j = 12345 AND k = 12345 AND l = 12345 AND m = 123456 AND n = 123456
      AND o = 10.12345 AND p = 10000.12 AND q = 10000.12 AND r = 10.12345 AND s = 10000.12
       AND t = -10000.12 AND u = 123.456 AND v = 123.456 AND w = 123.45 AND x = 12345.6789
        AND y = 12345.67 AND z = 12345.6789 AND aa = -12345.67;

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

SELECT a, b, c, d, e, f, g, h, i, j, k, l FROM characters
    WHERE a = 'a' AND b = 'a' AND c = 'chars' AND d = 'b' AND e = 'nchars' AND f = 'b' AND g = 'b'
     AND h = 'characters' AND i = 'ineedtwentyfivecharacters' AND j = 'variableness'
     AND k = 'nvariableness' AND l = 'varying characters';

CREATE TABLE numerics_signed (
    a TINYINT SIGNED,
    b TINYINT(10) UNSIGNED,
    c SMALLINT UNSIGNED,
    d SMALLINT(100) SIGNED,
    e INT SIGNED,
    f INT(64) UNSIGNED,
    g INTEGER UNSIGNED,
    h INTEGER(64) SIGNED,
    i BIGINT SIGNED,
    j BIGINT(100) UNSIGNED
);

INSERT INTO numerics_signed (a, b, c, d, e, f, g, h, i, j)
    VALUES(-1, +1, +10, -10, -100, +100, +100, -100, -1000, +1000);

SELECT a, b, c, d, e, f, g, h, i, j FROM numerics_signed;

SELECT a, b, c, d, e, f, g, h, i, j FROM numerics_signed
    WHERE a = -1 AND b = +1 AND c = +10 AND d = -10 AND e = -100
     AND f = +100 AND g = +100 AND h = -100 AND i = -1000 AND j = +1000;

CREATE TABLE temporal (
    a DATE,
    b DATETIME,
    c DATETIME(6),
    d TIME,
    e TIME(6),
    f TIMESTAMP,
    g TIMESTAMP(6) DEFAULT '1970-01-01 00:00:01.000000',
    h YEAR,
    i YEAR(4)
);

INSERT INTO temporal (a, b, c, d, e, f, g, h, i)
    VALUES('2016-09-15', '2015-01-24 15:22:06', '2015-01-24 15:22:06.002347', '15:22:06.002347',
        '15:22:06.002347', '2015-01-24 15:22:06', '2015-01-24 15:22:06.002347', '1993', '2006');

SELECT a, b, c, d, e, f, g, h, i FROM temporal;

SELECT a, b, c, d, e, f, g, h, i FROM temporal
    WHERE a = '2016-09-15' AND b = '2015-01-24 15:22:06' AND c = '2015-01-24 15:22:06.002347'
    AND d = '15:22:06.002347' AND e = '15:22:06.002347' AND f = '2015-01-24 15:22:06'
    AND g = '2015-01-24 15:22:06.002347' AND h = '1993' AND i = '2006';