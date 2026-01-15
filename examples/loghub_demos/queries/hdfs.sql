SELECT level, COUNT(*) as count FROM log GROUP BY level ORDER BY count DESC;
