SELECT node, COUNT(*) as count FROM log GROUP BY node ORDER BY count DESC LIMIT 10;
