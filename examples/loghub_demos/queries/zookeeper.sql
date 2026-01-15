SELECT level, component, COUNT(*) as count FROM log GROUP BY level, component ORDER BY count DESC LIMIT 10;
