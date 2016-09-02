CREATE TABLE users ( id INTEGER PRIMARY KEY, first_name VARCHAR(50), last_name VARCHAR(50), ssn VARCHAR(50), age INTEGER, sex VARCHAR(50) );
INSERT INTO users (id, first_name, last_name, ssn, age, sex) VALUES (1, 'Kurt', 'Cobain', '123456789', 27, 'M');
SELECT id, first_name, last_name, ssn, age, sex FROM users WHERE age = 27;

