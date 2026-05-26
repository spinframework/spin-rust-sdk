CREATE TABLE IF NOT EXISTS features (
    lat NUMBER,
    long NUMBER,
    name TEXT
);

DELETE FROM features;
INSERT INTO features(lat, long, name) VALUES (12, 20, 'Mount Hobbes');
INSERT INTO features(lat, long, name) VALUES (30, 8, 'Upper Rosie');
INSERT INTO features(lat, long, name) VALUES (14, 18, 'Slats'' Food Crater');
INSERT INTO features(lat, long, name) VALUES (25, 30, 'Forest of Smoke');
INSERT INTO features(lat, long, name) VALUES (22, 7, 'The Great Splodge');
INSERT INTO features(lat, long, name) VALUES (35, 21, 'Kiki Point');
INSERT INTO features(lat, long, name) VALUES (18, 19, 'Fang Rock');

CREATE TABLE IF NOT EXISTS route_notes(
    seq_no INTEGER PRIMARY KEY AUTOINCREMENT,
    lat NUMBER,
    long NUMBER,
    msg_text TEXT
);
