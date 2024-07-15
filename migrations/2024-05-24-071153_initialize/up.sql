CREATE TABLE covers (
    id INT(11) UNSIGNED NOT NULL,
    last_try DATETIME NOT NULL,
    provider TINYINT UNSIGNED,
    PRIMARY KEY (id)
);
