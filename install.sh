export OV_SOURCE="signage-client-install.s3.amazonaws.com"

EOL

echo "installing..."

# Download the other files
curl -O -sSf https://$OV_SOURCE/signaged
curl -O -sSf https://$OV_SOURCE/signaged.service
curl -O -sSf https://$OV_SOURCE/signage.json
#curl -O -sSf https://$OV_SOURCE/upgrade.sh

# Check if files were downloaded
if [ ! -f "signaged" ] || [ ! -f "signaged.service" ] || [ ! -f "signage.json" ]; then
    echo "Error: One or more files failed to download."
    exit 1
fi

# Move the signaged binary
if mv signaged /usr/bin/; then
    echo $HOME
else
    echo "Failed to move signaged to /usr/bin/"
    exit 1
fi

# Create the configuration directory
if mkdir -p "$HOME/.config/signage"; then
    echo "Created directory $HOME/.config/signage"
else
    echo "Failed to create directory $HOME/.config/signage"
    exit 1
fi

# Move the configuration file
if mv signage.json "$HOME/.config/signage/"; then
    echo "Moved signage.json to $HOME/.config/signage/"
else
    echo "Failed to move signage.json to .config/signage/"
    exit 1
fi

# Move the service file and enable the service
if mv signaged.service /etc/systemd/system/; then
    echo "Moved signaged.service to /etc/systemd/system/"
    systemctl daemon-reload
    systemctl enable signaged.service
    systemctl start signaged.service
    echo "Service signaged enabled and started"
else
    echo "Failed to move signaged.service to /etc/systemd/system/"
    exit 1
fi

echo "done!"