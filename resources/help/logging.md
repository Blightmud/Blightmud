# Logging

Blightmud is able to log your mud sessions to file. This includes everything you see when playing without the colors.

Files will be stored under: `$LOGDIR/<hostname>/<date-time>.log`

The following commands are available:
- `/start_log <hostname>` : Starts a log in the provided hostname folder
- `/stop_log`             : Stops logging

You may also setup blightmud to automatically log your playing. 
- `/set logging_enabled`          : Prints current setting
- `/set logging_enabled <on/off>` : Sets auto logging on or off

If enabled, blightmud will start logging once you connect to a mud.

##

***Timestamps***

You can enable timestamps in your session logs. When enabled, each line in the log file will be prefixed with a timestamp in `[HH:MM:SS]` format.

- `/set log_timestamps`          : Prints current setting
- `/set log_timestamps <on/off>` : Enables or disables timestamps in session logs

This setting takes effect immediately and applies to new log lines. Existing lines in the log are not affected.

***Note! Typed passwords and usernames will be logged, don't share your logs without thinking***
