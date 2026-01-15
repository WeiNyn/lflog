SELECT state, component, COUNT(*) as count FROM log GROUP BY state, component ORDER BY count DESC LIMIT 10;
