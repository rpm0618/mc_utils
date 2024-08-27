package net.minecraft.command;

import java.io.BufferedWriter;
import java.io.FileWriter;
import java.io.IOException;
import java.io.PrintWriter;
import java.io.StringWriter;
import java.net.Socket;
import java.text.SimpleDateFormat;
import java.util.ArrayList;
import java.util.Base64;
import java.util.Date;
import java.util.List;
import java.util.concurrent.ConcurrentLinkedQueue;

import net.minecraft.server.MinecraftServer;
import net.minecraft.util.BlockPos;
import net.minecraft.util.ChatComponentText;
import net.minecraft.util.JsonUtils;
import net.minecraft.util.LongHashMap;
import net.minecraft.util.LongHashMap.Entry;
import net.minecraft.world.World;
import net.minecraft.world.chunk.Chunk;
import net.minecraft.world.gen.ChunkProviderServer;

import com.google.gson.*;

/*
 * Place this file in net/minecraft/command/, and make the changes listed below
 *
 * In ServerCommandManager.java
 *
 * public ServerCommandManager()
 * {
 *     ...
 *     // CHUNK DEBUG START
 *     this.registerCommand(new CommandChunkDebug());
 *     // CHUNK DEBUG END
 *
 *     CommandBase.setAdminCommander(this);
 * }
 *
 * Imports (can be ignored if your ide does this for you)
 * In ChunkProviderServer.java
 *
 * ...
 * import org.apache.logging.log4j.Logger;
 * // CHUNK DEBUG START
 * import net.minecraft.command.CommandChunkDebug;
 * // CHUNK DEBUG END
 * ...
 *
 * In ChunkProviderGenerate.java
 *
 * ...
 * import net.minecraft.world.gen.structure.StructureOceanMonument;
 * // CHUNK DEBUG START
 * import net.minecraft.command.CommandChunkDebug;
 * // CHUNK DEBUG END
 * ...
 *
 * Make sure to also include the changes in comments on the onChunk* methods below
 */
public class CommandChunkDebug extends CommandBase {

    private static class ChunkMetadata {
        private static class ChunkMetadataJson {
            String stackTrace;
            String custom;
        }

        StackTraceElement[] stackTrace;
        String custom;

        public String toJson() {
            ChunkMetadataJson metadata = new ChunkMetadataJson();
            metadata.custom = custom;

            StringWriter sw = new StringWriter();
            PrintWriter pw = new PrintWriter(sw);
            for (int i = 0; i < this.stackTrace.length; i++) {
                pw.println(this.stackTrace[i].toString());
            }
            metadata.stackTrace = sw.toString();

            Gson gson = new Gson();
            return gson.toJson(metadata);
        }
    }

    private static class ChunkEntrySender implements Runnable {

        ConcurrentLinkedQueue<ChunkDebugEntry> entryQueue;

        ICommandSender sender;

        boolean running;

        int port;

        ChunkEntrySender(ConcurrentLinkedQueue<ChunkDebugEntry> entryQueue, ICommandSender sender, int port) {
            this.entryQueue = entryQueue;
            this.running = true;
            this.sender = sender;
            this.port = port;
        }

        @Override
        public void run() {
            try {
                System.out.println("PORT: " + this.port);
                Socket socket = new Socket("127.0.0.1", this.port);

                this.sender.addChatMessage(new ChatComponentText("Connected to chunk debug server"));

                PrintWriter out = new PrintWriter(socket.getOutputStream(), true);

                while(this.running) {
                    ChunkDebugEntry entry = this.entryQueue.poll();
                    while (entry != null) {
                        entry.write(out);
                        entry = this.entryQueue.poll();
                    }
                    out.flush();
                    Thread.sleep(50);
                }

                out.flush();
                out.close();

                socket.close();


                this.sender.addChatMessage(new ChatComponentText("Disconnected to chunk debug server"));
            } catch (IOException exception) {
                this.sender.addChatMessage(new ChatComponentText("Error talking to chunk debug server, check console"));
                CommandChunkDebug.chunkDebugEnabled = false;
                exception.printStackTrace();
            } catch (InterruptedException e) {
                e.printStackTrace();
            }
        }

        public void stop() {
            this.running = false;
        }

    }

    private static class ChunkDebugEntry {
        int x;
        int z;
        int tick;
        int world;
        String event;
        ChunkMetadata metadata;

        ChunkDebugEntry(int x, int z, int tick, int world, String event, ChunkMetadata metadata) {
            this.x = x;
            this.z = z;
            this.tick = tick;
            this.world = world;
            this.event = event;
            this.metadata = metadata;
        }

        public void write(PrintWriter pw) {
            String metadataStr = Base64.getEncoder().encodeToString(metadata.toJson().getBytes());

            // Avoid carriage return
            String out = x + "," + z + "," + tick + "," + world + "," + event + "," + metadataStr + "\n";
            pw.print(out);
        }
    }

    public static boolean chunkDebugEnabled = false;

    private static ConcurrentLinkedQueue<ChunkDebugEntry> entries = new ConcurrentLinkedQueue<ChunkDebugEntry>();

    public static int currentDimension = 0;

    private static ChunkEntrySender chunkSender = null;

    @Override
    public String getCommandName() {
        return "chunkDebug";
    }

    @Override
    public String getCommandUsage(ICommandSender sender) {
        return "/chunkDebug <start|stop|connect|disconnect> [port]";
    }

    @Override
    public void processCommand(ICommandSender sender, String[] args) throws CommandException {
        if (args.length < 1) {
            sender.addChatMessage(new ChatComponentText(getCommandUsage(sender)));
            return;
        }

        if (args[0].toLowerCase().equals("start")) {
            startChunkDebug(sender);
        } else if (args[0].toLowerCase().equals("stop")) {
            stopChunkDebug(sender);
        } else if (args[0].toLowerCase().equals("connect")) {
            int port = 20000;
            if (args.length >= 2) {
                try {
                    port = Integer.parseInt(args[1], 10);
                } catch (NumberFormatException e) {
                    sender.addChatMessage(new ChatComponentText(getCommandUsage(sender)));
                    return;
                }
            }
            connect(sender, port);
        } else if (args[0].toLowerCase().equals("disconnect")) {
            disconnect(sender);
        } else {
            sender.addChatMessage(new ChatComponentText(getCommandUsage(sender)));
            return;
        }
    }

    @Override
    public List<String> addTabCompletionOptions(ICommandSender sender, String[] args, BlockPos pos) {
        if (args.length == 1) {
            return getListOfStringsMatchingLastWord(args, new String[]{"start", "stop", "connect", "disconnect"});
        }
        return new ArrayList<String>();
    }

    @Override
    public int getRequiredPermissionLevel() {
        return 0;
    }

    private static void connect(ICommandSender sender, int port) {
        if (chunkDebugEnabled) {
            sender.addChatMessage(new ChatComponentText("Already enabled"));
            return;
        }

        chunkDebugEnabled = true;
        entries.clear();

        chunkSender = new ChunkEntrySender(entries, sender, port);
        new Thread(chunkSender, "Chunk Debug Thread").start();

        addAlreadyLoadedChunks();
    }

    private static void disconnect(ICommandSender sender) {
        chunkDebugEnabled = false;
        if (chunkSender != null) {
            chunkSender.stop();
        }
    }

    private static void startChunkDebug(ICommandSender sender) {
        if (chunkDebugEnabled) {
            sender.addChatMessage(new ChatComponentText("Already enabled"));
            return;
        }

        sender.addChatMessage(new ChatComponentText("Recording chunk events"));

        chunkDebugEnabled = true;
        entries.clear();

        addAlreadyLoadedChunks();
    }

    private static void addAlreadyLoadedChunks() {
        MinecraftServer server = MinecraftServer.getServer();
        for (int i = 0; i < server.worldServers.length; i++) {
            World world = server.worldServers[i];
            ChunkProviderServer provider = (ChunkProviderServer)(world.getChunkProvider());
            LongHashMap<Chunk> hashMap = provider.id2ChunkMap;
            for (int j = 0; j < hashMap.hashArray.length; j++) {
                Entry<Chunk> curEntry = hashMap.hashArray[j];
                while (curEntry != null) {
                    Chunk chunk = curEntry.getValue();
                    entries.add(new ChunkDebugEntry(chunk.xPosition, chunk.zPosition, getCurrentTick(), i, "ALREADY_LOADED", collectMetadata(null)));
                    curEntry = curEntry.nextEntry;
                }
            }
        }
    }

    private static void stopChunkDebug(ICommandSender sender) throws CommandException {
        chunkDebugEnabled = false;
        String fileName = "chunkDebug-" + new SimpleDateFormat("yyyy-MM-dd-HH-mm-ss-SSSS").format(new Date()) + ".csv";
        try {
            PrintWriter pw = new PrintWriter(new BufferedWriter(new FileWriter(fileName)));
            ChunkDebugEntry entry = entries.poll();
            while (entry != null) {
                entry.write(pw);
                entry = entries.poll();
            }

            pw.flush();
            pw.close();

            sender.addChatMessage(new ChatComponentText("Writing to file: " + fileName));
        } catch (Exception ex) {
            ex.printStackTrace();
            throw new CommandException(ex.getMessage());
        }
    }

    private static int getCurrentTick() {
        return MinecraftServer.getServer().getTickCounter();
    }

    private static ChunkMetadata collectMetadata(String custom) {
        StackTraceElement[] stackTrace = Thread.currentThread().getStackTrace();
        ChunkMetadata metadata = new ChunkMetadata();
        metadata.custom = custom;
        metadata.stackTrace = stackTrace;
        return metadata;
    }

    private static int getDimensionFromWorld(World world) {
        return world.provider.getDimensionId();
    }

    /*
     * In ChunkProviderServer.java
     *
     * public Chunk loadChunk(...)
     * {
     *     ...
     *             chunk = this.loadChunkFromFile(x, z);
     *             if (chunk == null)
     *             {
     *                 ...
     *             }
     *             // CHUNK DEBUG START
     *             else {
     *                 if (CommandChunkDebug.chunkDebugEnabled) {
     *                     CommandChunkDebug.onChunkLoaded(x, z, worldObj, null);
     *                 }
     *             }
     *             // CHUNK DEBUG END
     *     ...
     */
    public static void onChunkLoaded(int x, int z, World world, String custom) {
        entries.add(new ChunkDebugEntry(x, z, getCurrentTick(), getDimensionFromWorld(world), "LOADED", collectMetadata(custom)));
    }

    /*
     * In ChunkProviderServer.java
     *
     * public Chunk loadChunk(...)
     * {
     *     ...
     *             chunk = this.serverChunkGenerator.provideChunk(x, z);
     *             // CHUNK DEBUG START
     *             if (CommandChunkDebug.chunkDebugEnabled) {
     *                 CommandChunkDebug.onChunkGenerated(x, z, worldObj, null);
     *             }
     *             // CHUNK DEBUG END
     */
    public static void onChunkGenerated(int x, int z, World world, String custom) {
        entries.add(new ChunkDebugEntry(x, z, getCurrentTick(), getDimensionFromWorld(world), "GENERATED", collectMetadata(custom)));
    }

    /*
     * In ChunkProviderGenerate.java
     *
     * public void populate(...)
     * {
     *     // CHUNK DEBUG START
     *     if (CommandChunkDebug.chunkDebugEnabled) {
     *         CommandChunkDebug.onChunkPopulated(x, z, worldObj, null);
     *     }
     *     // CHUNK DEBUG END
     *
     *     BlockFalling.fallInstantly = true;
     *     ...
     */
    public static void onChunkPopulated(int x, int z, World world, String custom) {
        entries.add(new ChunkDebugEntry(x, z, getCurrentTick(), getDimensionFromWorld(world), "POPULATED", collectMetadata(custom)));
    }

    /*
     * In ChunkProviderServer.java
     *
     * In dropChunk(...) there are two lines that look like:
     *     this.droppedChunksSet.add(Long.valueOf(ChunkCoordIntPair.chunkXZ2Int(x, z)));
     *
     * Directly after each, place:
     *     // CHUNK DEBUG START
     *     if (CommandChunkDebug.chunkDebugEnabled) {
     *         CommandChunkDebug.onChunkUnloadScheduled(x, z, worldObj, null);
     *     }
     *     // CHUNK DEBUG END
     */
    public static void onChunkUnloadScheduled(int x, int z, World world, String custom) {
        entries.add(new ChunkDebugEntry(x, z, getCurrentTick(), getDimensionFromWorld(world), "UNLOAD_SCHEDULED", collectMetadata(custom)));
    }

    /*
     * In ChunkProviderServer.java
     *
     * public boolean unloadQueuedChunks()
     * {
     *     ...
     *             this.id2ChunkMap.remove(olong.longValue());
     *             this.loadedChunks.remove(chunk);
     *
     *              // CHUNK DEBUG START
     *              if (CommandChunkDebug.chunkDebugEnabled) {
     *                  CommandChunkDebug.onChunkUnloaded(chunk.xPosition, chunk.zPosition, worldObj, null);
     *              }
     *              // CHUNK DEBUG END
     *    ...
     */
    public static void onChunkUnloaded(int x, int z, World world, String custom) {
        entries.add(new ChunkDebugEntry(x, z, getCurrentTick(), getDimensionFromWorld(world), "UNLOADED", collectMetadata(custom)));
    }
}