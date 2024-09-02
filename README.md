# tmatebot
A simple Discord bot that can be used to create and close tmate connections.  
Allowed users set and use their own personal passwords to create connections.  
Created connections automatically close when there's no user left connected or if the timeout ends.

# .env
- TOKEN: The Discord bot's token.
- WHITELIST: List of Discord user IDs that are allowed to use the bot, each ID is separated by a comma with no whitespace.
- TIMEOUT: Default timeout value (example: `1h` to make connections automatically close after 1 hour).
