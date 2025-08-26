CREATE TABLE cats (
   name text not null,
   reign int4range not null
);

INSERT INTO cats (name, reign) VALUES
   ('Smoke', '[2005, 2013]'::int4range),
   ('Splodge', '[2005, 2019]'::int4range),
   ('Fang', '[2005, 2016]'::int4range),
   ('Kiki', '[2005, 2020]'::int4range),
   ('Slats', '[2005, 2021]'::int4range),
   ('Rosie', '[2021,)'::int4range),
   ('Hobbes', '[2021,)'::int4range)
;
