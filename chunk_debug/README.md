# 1.8 Chunk Debug

In order to use 1.8 chunk debug, you'll need a modified build of minecraft with a
custom command added to it. Use [MCP](http://www.modcoderpack.com/) to get a
buildable copy of the source code, and then follow the instructions in the
comments of `CommandChunkDebug.java` to add the custom command.

### Using the command
```
/chunkDebug start
```
Starts logging chunk events

```
/chunkDebug stop
```
Stop logging events, write them out to a file to be loaded with the viewer