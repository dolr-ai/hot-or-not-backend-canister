-- ─── IC Canister Snapshot Queries ─────────────────────────────────────────────
-- Database: ic_canisters.db
-- Run with: sqlite3 ic_canisters.db < queries.sql
.headers on
.mode column

-- ─── Platform Orchestrator Snapshot ──────────────────────────────────────────

-- Snapshot metadata (version, counts, timestamp)
SELECT key, value FROM po_metadata ORDER BY key;

-- All 16 subnet orchestrators (user_index canisters)
SELECT principal FROM po_subnet_orchestrators ORDER BY principal;

-- Controlled canister count
SELECT COUNT(*) AS controlled_canisters_total FROM po_controlled_canisters;

-- Sample of controlled canisters (first 20)
SELECT principal FROM po_controlled_canisters ORDER BY principal LIMIT 20;

-- Controlled canisters that are NOT already in the main canisters table (i.e. missing from the IC snapshot)
SELECT COUNT(*) AS not_in_ic_snapshot
FROM po_controlled_canisters p
WHERE NOT EXISTS (SELECT 1 FROM canisters c WHERE c.canister_id = p.principal);

-- Controlled canisters that ARE in the main canisters table, with their module hash
SELECT c.canister_id, c.module_hash, c.subnet_id
FROM canisters c
JOIN po_controlled_canisters p ON p.principal = c.canister_id
ORDER BY c.canister_id
LIMIT 20;

-- 3-level hierarchy: distinct canister_ids reachable from the root controllers,
-- each counted once, with how many canisters they control.
WITH known_principals AS (
    SELECT '7gaq2-4kttl-vtbt4-oo47w-igteo-cpk2k-57h3p-yioqe-wkawi-wz45g-jae' AS id, 'sns admin' AS name
    UNION ALL SELECT 'zg7n3-345by-nqf6o-3moz4-iwxql-l6gko-jqdz2-56juu-ja332-unymr-fqe', 'sns proposal submitter'
    UNION ALL SELECT 'efsfj-sqaaa-aaaap-qatwa-cai',  'configuration'
    UNION ALL SELECT 'jwktp-qyaaa-aaaag-abcfa-cai',  'data_backup'
    UNION ALL SELECT '74zq4-iqaaa-aaaam-ab53a-cai',  'platform_orchestrator'
    UNION ALL SELECT 'vyatz-hqaaa-aaaam-qauea-cai',  'webapp frontend'
    UNION ALL SELECT '6wcax-haaaa-aaaaq-aaava-cai',  'DOLR AI Governance'
    UNION ALL SELECT '6dfr2-giaaa-aaaaq-aaawq-cai',  'DOLR AI DOLR Index'
    UNION ALL SELECT '6rdgd-kyaaa-aaaaq-aaavq-cai',  'DOLR AI DOLR Ledger'
    UNION ALL SELECT '67bll-riaaa-aaaaq-aaauq-cai',  'DOLR AI Root'
    UNION ALL SELECT '6eexo-lqaaa-aaaaq-aaawa-cai',  'DOLR AI Swap'
    UNION ALL SELECT 'dc47w-kaaaa-aaaak-qav3q-cai',  'individual_user_template'
    UNION ALL SELECT 'rimrc-piaaa-aaaao-aaljq-cai',  'user_index'
    UNION ALL SELECT 'ivkka-7qaaa-aaaas-qbg3q-cai',  'user_info_service'
    UNION ALL SELECT 'h2jgv-ayaaa-aaaas-qbh4a-cai',  'rate_limits'
    UNION ALL SELECT 'gxhc3-pqaaa-aaaas-qbh3q-cai',  'user_post_service'
),
level1 AS (
    SELECT DISTINCT canister_id FROM controllers
    WHERE controller IN (SELECT id FROM known_principals)
),
level2 AS (
    SELECT DISTINCT co.canister_id FROM controllers co
    JOIN level1 l ON co.controller = l.canister_id
),
level3 AS (
    SELECT DISTINCT co.canister_id FROM controllers co
    JOIN level2 l ON co.controller = l.canister_id
),
all_canisters AS (
    SELECT canister_id FROM level1
    UNION
    SELECT canister_id FROM level2
    UNION
    SELECT canister_id FROM level3
)
SELECT
    ac.canister_id AS controller_id,
    kp.name AS known_as,
    COUNT(co.canister_id) AS count,
    GROUP_CONCAT(DISTINCT kp2.name) AS controlled_by_known
FROM all_canisters ac
JOIN controllers co ON co.controller = ac.canister_id
LEFT JOIN known_principals kp ON kp.id = ac.canister_id
LEFT JOIN controllers ctrl ON ctrl.canister_id = ac.canister_id
LEFT JOIN known_principals kp2 ON kp2.id = ctrl.controller
GROUP BY ac.canister_id
ORDER BY count DESC;

WITH known_principals AS (
    SELECT '7gaq2-4kttl-vtbt4-oo47w-igteo-cpk2k-57h3p-yioqe-wkawi-wz45g-jae' AS id, 'sns admin' AS name
    UNION ALL SELECT 'zg7n3-345by-nqf6o-3moz4-iwxql-l6gko-jqdz2-56juu-ja332-unymr-fqe', 'sns proposal submitter'
    UNION ALL SELECT 'efsfj-sqaaa-aaaap-qatwa-cai',  'configuration'
    UNION ALL SELECT 'jwktp-qyaaa-aaaag-abcfa-cai',  'data_backup'
    UNION ALL SELECT '74zq4-iqaaa-aaaam-ab53a-cai',  'platform_orchestrator'
    UNION ALL SELECT 'vyatz-hqaaa-aaaam-qauea-cai',  'webapp frontend'
    UNION ALL SELECT '6wcax-haaaa-aaaaq-aaava-cai',  'DOLR AI Governance'
    UNION ALL SELECT '6dfr2-giaaa-aaaaq-aaawq-cai',  'DOLR AI DOLR Index'
    UNION ALL SELECT '6rdgd-kyaaa-aaaaq-aaavq-cai',  'DOLR AI DOLR Ledger'
    UNION ALL SELECT '67bll-riaaa-aaaaq-aaauq-cai',  'DOLR AI Root'
    UNION ALL SELECT '6eexo-lqaaa-aaaaq-aaawa-cai',  'DOLR AI Swap'
    UNION ALL SELECT 'dc47w-kaaaa-aaaak-qav3q-cai',  'individual_user_template'
    UNION ALL SELECT 'rimrc-piaaa-aaaao-aaljq-cai',  'user_index'
    UNION ALL SELECT 'ivkka-7qaaa-aaaas-qbg3q-cai',  'user_info_service'
    UNION ALL SELECT 'h2jgv-ayaaa-aaaas-qbh4a-cai',  'rate_limits'
    UNION ALL SELECT 'gxhc3-pqaaa-aaaas-qbh3q-cai',  'user_post_service'
),
level1 AS (
    SELECT DISTINCT canister_id FROM controllers
    WHERE controller IN (SELECT id FROM known_principals)
),
level2 AS (
    SELECT DISTINCT co.canister_id FROM controllers co
    JOIN level1 l ON co.controller = l.canister_id
),
level3 AS (
    SELECT DISTINCT co.canister_id FROM controllers co
    JOIN level2 l ON co.controller = l.canister_id
),
all_canisters AS (
    SELECT canister_id FROM level1
    UNION
    SELECT canister_id FROM level2
    UNION
    SELECT canister_id FROM level3
)
SELECT
    ac.canister_id AS controller_id,
    kp.name AS known_as,
    COUNT(co.canister_id) AS count
FROM all_canisters ac
JOIN controllers co ON co.controller = ac.canister_id
LEFT JOIN known_principals kp ON kp.id = ac.canister_id
GROUP BY ac.canister_id
ORDER BY count DESC;

-- Total distinct canisters across all 3 levels.
WITH known_principals AS (
    SELECT '7gaq2-4kttl-vtbt4-oo47w-igteo-cpk2k-57h3p-yioqe-wkawi-wz45g-jae' AS id, 'sns admin' AS name
    UNION ALL SELECT 'zg7n3-345by-nqf6o-3moz4-iwxql-l6gko-jqdz2-56juu-ja332-unymr-fqe', 'sns proposal submitter'
    UNION ALL SELECT 'efsfj-sqaaa-aaaap-qatwa-cai',  'configuration'
    UNION ALL SELECT 'jwktp-qyaaa-aaaag-abcfa-cai',  'data_backup'
    UNION ALL SELECT '74zq4-iqaaa-aaaam-ab53a-cai',  'platform_orchestrator'
    UNION ALL SELECT 'vyatz-hqaaa-aaaam-qauea-cai',  'webapp frontend'
    UNION ALL SELECT '6wcax-haaaa-aaaaq-aaava-cai',  'DOLR AI Governance'
    UNION ALL SELECT '6dfr2-giaaa-aaaaq-aaawq-cai',  'DOLR AI DOLR Index'
    UNION ALL SELECT '6rdgd-kyaaa-aaaaq-aaavq-cai',  'DOLR AI DOLR Ledger'
    UNION ALL SELECT '67bll-riaaa-aaaaq-aaauq-cai',  'DOLR AI Root'
    UNION ALL SELECT '6eexo-lqaaa-aaaaq-aaawa-cai',  'DOLR AI Swap'
    UNION ALL SELECT 'dc47w-kaaaa-aaaak-qav3q-cai',  'individual_user_template'
    UNION ALL SELECT 'rimrc-piaaa-aaaao-aaljq-cai',  'user_index'
    UNION ALL SELECT 'ivkka-7qaaa-aaaas-qbg3q-cai',  'user_info_service'
    UNION ALL SELECT 'h2jgv-ayaaa-aaaas-qbh4a-cai',  'rate_limits'
    UNION ALL SELECT 'gxhc3-pqaaa-aaaas-qbh3q-cai',  'user_post_service'
),
level1 AS (
    SELECT DISTINCT canister_id FROM controllers
    WHERE controller IN (SELECT id FROM known_principals)
),
level2 AS (
    SELECT DISTINCT co.canister_id FROM controllers co
    JOIN level1 l ON co.controller = l.canister_id
),
level3 AS (
    SELECT DISTINCT co.canister_id FROM controllers co
    JOIN level2 l ON co.controller = l.canister_id
),
all_canisters AS (
    SELECT canister_id FROM level1
    UNION
    SELECT canister_id FROM level2
    UNION
    SELECT canister_id FROM level3
)
SELECT COUNT(*) AS total_canisters FROM all_canisters;

-- External controllers (not in roots, not in hierarchy) and how many hierarchy canisters they control.
WITH roots AS (
    SELECT '7gaq2-4kttl-vtbt4-oo47w-igteo-cpk2k-57h3p-yioqe-wkawi-wz45g-jae' AS id, 'sns admin' AS name
    UNION ALL SELECT 'zg7n3-345by-nqf6o-3moz4-iwxql-l6gko-jqdz2-56juu-ja332-unymr-fqe', 'sns proposal submitter'
    UNION ALL SELECT 'efsfj-sqaaa-aaaap-qatwa-cai',  'configuration'
    UNION ALL SELECT 'jwktp-qyaaa-aaaag-abcfa-cai',  'data_backup'
    UNION ALL SELECT '74zq4-iqaaa-aaaam-ab53a-cai',  'platform_orchestrator'
    UNION ALL SELECT 'vyatz-hqaaa-aaaam-qauea-cai',  'webapp frontend'
    UNION ALL SELECT '6wcax-haaaa-aaaaq-aaava-cai',  'DOLR AI Governance'
    UNION ALL SELECT '6dfr2-giaaa-aaaaq-aaawq-cai',  'DOLR AI DOLR Index'
    UNION ALL SELECT '6rdgd-kyaaa-aaaaq-aaavq-cai',  'DOLR AI DOLR Ledger'
    UNION ALL SELECT '67bll-riaaa-aaaaq-aaauq-cai',  'DOLR AI Root'
    UNION ALL SELECT '6eexo-lqaaa-aaaaq-aaawa-cai',  'DOLR AI Swap'
    UNION ALL SELECT 'dc47w-kaaaa-aaaak-qav3q-cai',  'individual_user_template'
    UNION ALL SELECT 'rimrc-piaaa-aaaao-aaljq-cai',  'user_index'
    UNION ALL SELECT 'ivkka-7qaaa-aaaas-qbg3q-cai',  'user_info_service'
    UNION ALL SELECT 'h2jgv-ayaaa-aaaas-qbh4a-cai',  'rate_limits'
    UNION ALL SELECT 'gxhc3-pqaaa-aaaas-qbh3q-cai',  'user_post_service'
),
level1 AS (
    SELECT DISTINCT canister_id FROM controllers
    WHERE controller IN (SELECT id FROM roots)
),
level2 AS (
    SELECT DISTINCT co.canister_id FROM controllers co
    JOIN level1 l ON co.controller = l.canister_id
),
level3 AS (
    SELECT DISTINCT co.canister_id FROM controllers co
    JOIN level2 l ON co.controller = l.canister_id
),
all_canisters AS (
    SELECT canister_id FROM level1
    UNION
    SELECT canister_id FROM level2
    UNION
    SELECT canister_id FROM level3
)
SELECT
    co.controller AS external_controller,
    r.name AS controller_known_as,
    COUNT(DISTINCT co.canister_id) AS controlled_count
FROM controllers co
JOIN all_canisters ac ON co.canister_id = ac.canister_id
LEFT JOIN roots r ON r.id = co.controller
WHERE co.controller NOT IN (SELECT id FROM roots)
  AND co.controller NOT IN (SELECT canister_id FROM all_canisters)
GROUP BY co.controller
ORDER BY controlled_count DESC;

-- External controllers (not in roots, not in hierarchy) and count of hierarchy canisters they control.
WITH roots AS (
    SELECT '7gaq2-4kttl-vtbt4-oo47w-igteo-cpk2k-57h3p-yioqe-wkawi-wz45g-jae' AS id, 'sns admin' AS name
    UNION ALL SELECT 'zg7n3-345by-nqf6o-3moz4-iwxql-l6gko-jqdz2-56juu-ja332-unymr-fqe', 'sns proposal submitter'
    UNION ALL SELECT 'efsfj-sqaaa-aaaap-qatwa-cai',  'configuration'
    UNION ALL SELECT 'jwktp-qyaaa-aaaag-abcfa-cai',  'data_backup'
    UNION ALL SELECT '74zq4-iqaaa-aaaam-ab53a-cai',  'platform_orchestrator'
    UNION ALL SELECT 'vyatz-hqaaa-aaaam-qauea-cai',  'webapp frontend'
    UNION ALL SELECT '6wcax-haaaa-aaaaq-aaava-cai',  'DOLR AI Governance'
    UNION ALL SELECT '6dfr2-giaaa-aaaaq-aaawq-cai',  'DOLR AI DOLR Index'
    UNION ALL SELECT '6rdgd-kyaaa-aaaaq-aaavq-cai',  'DOLR AI DOLR Ledger'
    UNION ALL SELECT '67bll-riaaa-aaaaq-aaauq-cai',  'DOLR AI Root'
    UNION ALL SELECT '6eexo-lqaaa-aaaaq-aaawa-cai',  'DOLR AI Swap'
    UNION ALL SELECT 'dc47w-kaaaa-aaaak-qav3q-cai',  'individual_user_template'
    UNION ALL SELECT 'rimrc-piaaa-aaaao-aaljq-cai',  'user_index'
    UNION ALL SELECT 'ivkka-7qaaa-aaaas-qbg3q-cai',  'user_info_service'
    UNION ALL SELECT 'h2jgv-ayaaa-aaaas-qbh4a-cai',  'rate_limits'
    UNION ALL SELECT 'gxhc3-pqaaa-aaaas-qbh3q-cai',  'user_post_service'
),
level1 AS (
    SELECT DISTINCT canister_id, 1 AS level FROM controllers
    WHERE controller IN (SELECT id FROM roots)
),
level2 AS (
    SELECT DISTINCT co.canister_id, 2 AS level FROM controllers co
    JOIN level1 l ON co.controller = l.canister_id
),
level3 AS (
    SELECT DISTINCT co.canister_id, 3 AS level FROM controllers co
    JOIN level2 l ON co.controller = l.canister_id
),
all_canisters AS (
    SELECT canister_id, level FROM level1
    UNION
    SELECT canister_id, level FROM level2
    UNION
    SELECT canister_id, level FROM level3
)
SELECT
    co.controller AS external_controller,
    ac.level,
    COUNT(DISTINCT co.canister_id) AS controlled_count
FROM controllers co
JOIN all_canisters ac ON co.canister_id = ac.canister_id
WHERE co.controller NOT IN (SELECT id FROM roots)
  AND co.controller NOT IN (SELECT canister_id FROM all_canisters)
GROUP BY co.controller, ac.level
ORDER BY co.controller, ac.level;

-- External controllers that directly control any of our known principals.
WITH known_principals AS (
    SELECT '7gaq2-4kttl-vtbt4-oo47w-igteo-cpk2k-57h3p-yioqe-wkawi-wz45g-jae' AS id, 'sns admin' AS name
    UNION ALL SELECT 'zg7n3-345by-nqf6o-3moz4-iwxql-l6gko-jqdz2-56juu-ja332-unymr-fqe', 'sns proposal submitter'
    UNION ALL SELECT 'efsfj-sqaaa-aaaap-qatwa-cai',  'configuration'
    UNION ALL SELECT 'jwktp-qyaaa-aaaag-abcfa-cai',  'data_backup'
    UNION ALL SELECT '74zq4-iqaaa-aaaam-ab53a-cai',  'platform_orchestrator'
    UNION ALL SELECT 'vyatz-hqaaa-aaaam-qauea-cai',  'webapp frontend'
    UNION ALL SELECT '6wcax-haaaa-aaaaq-aaava-cai',  'DOLR AI Governance'
    UNION ALL SELECT '6dfr2-giaaa-aaaaq-aaawq-cai',  'DOLR AI DOLR Index'
    UNION ALL SELECT '6rdgd-kyaaa-aaaaq-aaavq-cai',  'DOLR AI DOLR Ledger'
    UNION ALL SELECT '67bll-riaaa-aaaaq-aaauq-cai',  'DOLR AI Root'
    UNION ALL SELECT '6eexo-lqaaa-aaaaq-aaawa-cai',  'DOLR AI Swap'
    UNION ALL SELECT 'dc47w-kaaaa-aaaak-qav3q-cai',  'individual_user_template'
    UNION ALL SELECT 'rimrc-piaaa-aaaao-aaljq-cai',  'user_index'
    UNION ALL SELECT 'ivkka-7qaaa-aaaas-qbg3q-cai',  'user_info_service'
    UNION ALL SELECT 'h2jgv-ayaaa-aaaas-qbh4a-cai',  'rate_limits'
    UNION ALL SELECT 'gxhc3-pqaaa-aaaas-qbh3q-cai',  'user_post_service'
)
SELECT
    co.controller AS external_controller,
    co.canister_id AS known_canister,
    kp.name AS known_canister_name,
    GROUP_CONCAT(DISTINCT kp2.name) AS also_controlled_by
FROM controllers co
JOIN known_principals kp ON kp.id = co.canister_id
LEFT JOIN controllers co2 ON co2.canister_id = co.canister_id
    AND co2.controller != co.controller
LEFT JOIN known_principals kp2 ON kp2.id = co2.controller
WHERE co.controller NOT IN (SELECT id FROM known_principals)
GROUP BY co.controller, co.canister_id
ORDER BY co.canister_id, co.controller;
