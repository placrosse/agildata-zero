CREATE TABLE users ( id INTEGER PRIMARY KEY, first_name VARCHAR(50), last_name VARCHAR(50), ssn VARCHAR(50), age INTEGER, sex VARCHAR(50) );
INSERT INTO users (id, first_name, last_name, ssn, age, sex) VALUES (1, 'Kurt', 'Cobain', '123456789', 27, 'M');
INSERT INTO users (id, first_name, last_name, ssn, age, sex) VALUES (2, 'Jenny', 'Jones', '123356789', 27, 'F');
INSERT INTO users (id, first_name, last_name, ssn, age, sex) VALUES (3, 'Leonard', 'Hofstadder', '123156789', 39, 'M');
INSERT INTO users (id, first_name, last_name, ssn, age, sex) VALUES (4, 'Maggie', 'McGarry', '123456689', 81, 'F');
INSERT INTO users (id, first_name, last_name, ssn, age, sex) VALUES (5, 'Barry', 'Kripke', '123452789', 34, 'M');
INSERT INTO users (id, first_name, last_name, ssn, age, sex) VALUES (6, 'Amy', 'Farrah-Fowler', '123156789', 39, 'F');
INSERT INTO users (id, first_name, last_name, ssn, age, sex) VALUES (7, 'Sheldon', 'Cooper', '123453789', 35, 'M');
INSERT INTO users (id, first_name, last_name, ssn, age, sex) VALUES (8, 'Bernadette', 'Rostenkowski-Walowitz', '133456789', 32, 'F');
INSERT INTO users (id, first_name, last_name, ssn, age, sex) VALUES (9, 'David "Ginsano Bianco"', 'Lister', '223456789', 42, 'M');
INSERT INTO users (id, first_name, last_name, ssn, age, sex) VALUES (10, 'Arnold Judas', 'Rimmer', '623456789', 3000043, 'M');
SELECT id, first_name, last_name, ssn, age, sex FROM users;
SELECT id, first_name, last_name, ssn, age, sex FROM users WHERE first_name = 'Barry';
UPDATE users SET id = id + 100, first_name = 'Janis' WHERE id = 1;
SELECT id, first_name, last_name, ssn, age, sex FROM users WHERE first_name = 'Janis';
SELECT id, first_name, last_name, ssn, age, sex FROM users ORDER BY id DESC LIMIT 1;
SELECT id, first_name, last_name, ssn, age, sex FROM users ORDER BY id ASC LIMIT 2;

