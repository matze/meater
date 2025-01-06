# MEATER reader

A small helper crate to connect to and read data from a [MEATER](https://www.meater.com) smart
thermometer as well as a little Raspberry Pi Zero application to display
the temperature and battery level.


## Raspberry Pi application

Install the following `meater.service` into `/etc/systemd/system/meater.service`


```
[Unit]
Description=MEATER service
After=bluetooth.target

[Service]
ExecStart=/usr/bin/meater

[Install]
WantedBy=multi-user.target
```

to start the application as a systemd service automatically. In addition, edit
`/etc/bluetooth/main.conf` and change

```
DiscoverableTimeout = 0
```

to avoid losing the device after three minutes.


## Acknowledgements

Temperature conversion taken from the reverse engineering efforts by [Nathan
Faber](https://github.com/nathanfaber/meaterble).


## License

[MIT](./LICENSE)
