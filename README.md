# Epicinium Server

An asynchronous multiplayer server for [Epicinium](https://epicinium.nl).

Epicinium is a multiplayer strategy game with simultaneous turns,
released on October 12th, 2020 and open-sourced on the same day.
It is available as a free game on [Steam](https://store.steampowered.com/app/1286730/Epicinium/) and [itch.io](https://abunchofhacks.itch.io/epicinium).

Epicinium is being developed by [A Bunch of Hacks](https://abunchofhacks.coop),
a worker cooperative for video game and software development from the Netherlands.
Contact us at [info@epicinium.nl](mailto:info@epicinium.nl).

## Support

Epicinium is and will remain free software. If you wish to support Epicinium and A Bunch of Hacks, you have the option to [name-your-own-price](https://abunchofhacks.itch.io/epicinium/purchase) or [buy the game's soundtrack](https://store.steampowered.com/app/1442600/Epicinium__Extended_Soundtrack/).

## Contents

*  `src/bin/server.rs` is the entry point for the server executable
*  `src/server` contains code for handling incoming connections and starting games
*  `src/server/client` contains subtasks that handle communications to and from a single connected game client
*  `src/common` contains constants and utility functions
*  `src/logic` contains idiomatic access to the raw bindings provided by [epicinium_lib](https://github.com/abunchofhacks/Epicinium-lib-rs)
*  `logs` is where all log files are stored
*  `recordings` is where recordings of games in progress are stored

## License

The Epicinium server was created by [A Bunch of Hacks](https://abunchofhacks.coop).
It is made available to you under the AGPL-3.0 License,
as specified in `LICENSE.txt`.

The Epicinium server is free software; you can redistribute it and/or modify it under the terms of the GNU Affero General Public License (AGPL) as published by the Free Software Foundation; either version 3 of the License, or (at your option) any later version.

The Epicinium server is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details.

## Related repositories

*  [epicinium_lib](https://github.com/abunchofhacks/Epicinium-lib-rs), Rust bindings for the C++ library that implements Epicinium gameplay logic
*  [Epicinium](https://github.com/abunchofhacks/Epicinium), the full source code for Epicinium
*  [Epicinium documentation](https://github.com/abunchofhacks/epicinium-documentation), which includes a wiki and a tutorial for Epicinium
*  [Epicinium-NeuralNewt](https://github.com/abunchofhacks/Epicinium-NeuralNewt), a libtorch framework for training (partially) convolutional neural networks to play Epicinium via NeuralNewt, a parameterized decision tree AI, with evolutionary training techniques
