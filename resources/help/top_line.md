# Top line method

This method allows you to control the content of the top line. By default, the top line shows the host of the server you are connected
to as well as all of the protocols that the server supports. To hide the top bar completely, set toggle the `hide_topbar` setting. See `/help settings`.

##

***blight.top_line(line)***
Sets the content of the top line. The content will be integrated into the bar.
- `line`    The line you want to print. Passing `nil` will reset the bar to the default content.
