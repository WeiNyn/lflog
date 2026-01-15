SELECT component, COUNT(*) as count FROM log GROUP BY component ORDER BY count DESC LIMIT 10;
