# ibc-tests-ics20

Simple repo showing how to use ts-relayer as a library to test cw20-ics20 contract

## Setup

Ensure you have node 14+ (16+ recommended):

```
node --version
```

Then install via npm as typical:

```
npm install
```

## Development

Build the source:

```
npm run build
```

Clean it up with prettier and eslint:

```
npm run fix
```

## Testing

This actually runs the test codes on contracts. To do so, we need to start two blockchains
in the background and then run the process. This requires that you have docker installed
and running on your local machine. If you don't, please do that first before running the scripts.
(Also, they only work on Linux and MacOS... sorry Windows folks, you are welcome to PR an equivalent).

Terminal 1:

```
cd scripts/gaia
./start.sh
```

Terminal 2:

```
cd scripts/wasmd
./start.sh
```

If those start properly, you should see a series of `executed block` messages. If they fail, check `debug.log`
in that directory for full log messages.

Terminal 3:

```
npm run test
```

You may run and re-run tests many times. When you are done with it and want to free up some system resources
(stop running two blockchains in the background), you need to run these commands to stop them properly:

```
./scripts/gaia/stop.sh
./scripts/wasmd/stop.sh
```
