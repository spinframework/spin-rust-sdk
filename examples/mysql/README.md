# Setting up MySQL for the sample

```bash
docker run --rm -h 127.0.0.1 -p 3306:3306 -e MYSQL_DATABASE=spin_dev -e MYSQL_ROOT_PASSWORD=spin mysql
docker exec -it ecstatic_khayyam bash # use your container instance name

# At container bash prompt
mysql -p  --database spin_dev # enter `spin`

# At mysql prompt
CREATE TABLE pets (id INT PRIMARY KEY, name VARCHAR(100) NOT NULL, prey VARCHAR(100), is_finicky BOOL NOT NULL);
INSERT INTO pets VALUES (1, 'Splodge', NULL, false);
INSERT INTO pets VALUES (2, 'Kiki', 'Cicadas', false);
INSERT INTO pets VALUES (3, 'Slats', 'Temptations', true);
CREATE USER 'spin' IDENTIFIED BY 'spin';
GRANT CREATE, ALTER, DROP, INSERT, UPDATE, DELETE, SELECT ON spin_dev.pets TO 'spin';
```
