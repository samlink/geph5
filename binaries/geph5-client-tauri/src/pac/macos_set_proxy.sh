#!/bin/bash

proxy_server="127.0.0.1" # 例如: 192.168.1.100 或 proxy.example.com
port="9910"  

# AppleScript to request administrator privileges and run the script
osascript -e 'do shell script "
# Get the list of network services
network_services=$(networksetup -listallnetworkservices | tail -n +2)

# Loop through each line using read
while IFS= read -r service; do
  # Set HTTP and HTTPS proxies
  networksetup -setwebproxy \"$service\" \"$proxy_server\" $port
  networksetup -setwebproxystate \"$service\" on
  networksetup -setsecurewebproxy \"$service\" \"$proxy_server\" $port
  networksetup -setsecurewebproxystate \"$service\" on
done <<< \"$network_services\"
" with administrator privileges'

# Note: You need to define the variables $proxy_server and $port before running the script
