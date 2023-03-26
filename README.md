<h1 align="center">Forza Telemetry</h1>

<p align="center">Blazingly fast 💥 UDP server to receive all telemetry data from Forza games and write them to a CSV file.</p>

<br/>
<br/>

# Installation (compiling)
```bash
# Clone the repo
git clone https://github.com/0x20F/forza-telemetry

# Go into the directory
cd forza-telemetry

# Install the package on your system (this will build everything and it might take a while)
cargo install --path .

# Run
forza-telemetry
```

<br/>

# Running
```bash
# Running with no params will output into "output.csv"
forza-telemetry

# Running with the -f param will output into a file of your choosing
forza-telemetry -f my-custom-telemetry.csv

# Use this for help
forza-telemetry --help
```

<br/>

# The future
Right now it only supports FM7, maybe FH5 soon enough. The idea is to drag FM8 into this as soon as it releases given the revamped physics engine, they've got to have a lot more data. Other than that there's no plans to add support for the older games yet. If you have a need and requirement for it, make an issue and yell at me 😄.