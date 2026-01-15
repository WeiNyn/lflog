SELECT program, COUNT(*) as count FROM log GROUP BY program ORDER BY count DESC LIMIT 10;
