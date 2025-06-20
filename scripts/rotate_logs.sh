#!/bin/bash
# Log rotation script

LOG_DIR="logs"
MAX_SIZE="10M"
MAX_AGE="7"

find "$LOG_DIR" -name "*.log" -size +$MAX_SIZE -exec gzip {} \;
find "$LOG_DIR" -name "*.log.gz" -mtime +$MAX_AGE -delete

echo "$(date): Log rotation completed" >> "$LOG_DIR/rotation.log"
