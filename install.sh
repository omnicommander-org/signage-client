cat << EOL

   _ \   \  |   \ | _ _| \ \   / _ _|   __| _ _|   _ \   \ |
  (   | |\/ |  .  |   |   \ \ /    |  \__ \   |   (   | .  |
 \___/ _|  _| _|\_| ___|   \_/   ___| ____/ ___| \___/ _|\_|

EOL

echo "installing..."

# Download the other files
curl -O -sSf https://$OV_SOURCE/signaged
curl -O -sSf https://$OV_SOURCE/signaged.service
curl -O -sSf https://$OV_SOURCE/signage.json
curl -O -sSf https://$OV_SOURCE/upgrade.sh

mv signaged /usr/bin/

mkdir -p $HOME/.config/signage
mv signage.json $HOME/.config/signage/

mv signaged.service /etc/systemd/system
systemctl enable signaged
systemctl start signaged

echo "done!"
