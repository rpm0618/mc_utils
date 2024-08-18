package net.minecraft.command;

import java.io.BufferedWriter;
import java.io.FileWriter;
import java.io.PrintWriter;
import java.io.StringWriter;
import java.text.SimpleDateFormat;
import java.util.ArrayList;
import java.util.Base64;
import java.util.Date;

import net.minecraft.server.MinecraftServer;
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
 *     //CHUNK DEBUG START
 *     this.registerCommand(new CommandChunkDebug());
 *     //CHUNK DEBUG END
 *
 *     CommandBase.setAdminCommander(this);
 * }
 *
 * Imports (can be ignored if your ide does this for you)
 * In ChunkProviderServer.java
 *
 * ...
 * import org.apache.logging.log4j.Logger;
 * //CHUNK DEBUG START
 * import net.minecraft.command.CommandChunkDebug;
 * //CHUNK DEBUG END
 * ...
 *
 * In ChunkProviderGenerate.java
 *
 * ...
 * import net.minecraft.world.gen.structure.StructureOceanMonument;
 * //CHUNK DEBUG START
 * import net.minecraft.command.CommandChunkDebug;
 * //CHUNK DEBUG END
 * ...
 *
 * Make sure to also include the changes in comments on the onChunk* methods below
 */
public class CommandChunkDebug extends CommandBase {

    private static class ChunkMetadata {
        String stackTrace;
        String custom;

        public String toJson() {
            Gson gson = new Gson();
            return gson.toJson(this);
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
    }

    public static boolean chunkDebugEnabled = false;

    private static ArrayList<ChunkDebugEntry> entries = new ArrayList<ChunkDebugEntry>();

    private static int currentDimension = 0;

    @Override
    public String getCommandName() {
        return "chunkDebug";
    }

    @Override
    public String getCommandUsage(ICommandSender sender) {
        return "/chunkDebug <start|stop>";
    }

    @Override
    public void processCommand(ICommandSender sender, String[] args) throws CommandException {
        if (args.length < 1) {
            sender.addChatMessage(new ChatComponentText(getCommandUsage(sender)));
            return;
        }

        if (args[0].toLowerCase().equals("start")) {
            startChunkDebug();
        } else if (args[0].toLowerCase().equals("stop")) {
            stopChunkDebug(sender);
        } else {
            sender.addChatMessage(new ChatComponentText(getCommandUsage(sender)));
            return;
        }
    }

    @Override
    public int getRequiredPermissionLevel() {
        return 0;
    }

    public static void setCurrentDimension(int dimension) {
        CommandChunkDebug.currentDimension = dimension;
    }

    private static void startChunkDebug() {
        chunkDebugEnabled = true;
        entries.clear();

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
            for (int i = 0; i < entries.size(); i++) {
                ChunkDebugEntry entry = entries.get(i);
                String metadata = Base64.getEncoder().encodeToString(entry.metadata.toJson().getBytes());
                pw.println(entry.x + "," + entry.z + "," + entry.tick + "," + entry.world + "," + entry.event + "," + metadata);
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
        StringWriter sw = new StringWriter();
        PrintWriter pw = new PrintWriter(sw);
        for (int i = 0; i < stackTrace.length; i++) {
            pw.println(stackTrace[i].toString());
        }
        ChunkMetadata metadata = new ChunkMetadata();
        metadata.custom = custom;
        metadata.stackTrace = sw.toString();
        return metadata;
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
     *                     CommandChunkDebug.onChunkLoaded(x, z, null);
     *                 }
     *             }
     *             // CHUNK DEBUG END
     *     ...
     */
    public static void onChunkLoaded(int x, int z, String custom) {
        entries.add(new ChunkDebugEntry(x, z, getCurrentTick(), currentDimension, "LOADED", collectMetadata(custom)));
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
     *                 CommandChunkDebug.onChunkGenerated(x, z, null);
     *             }
     *             // CHUNK DEBUG END
     */
    public static void onChunkGenerated(int x, int z, String custom) {
        entries.add(new ChunkDebugEntry(x, z, getCurrentTick(), currentDimension, "GENERATED", collectMetadata(custom)));
    }

    /*
     * In ChunkProviderGenerate.java
     *
     * public void populate(...)
     * {
     *     // CHUNK DEBUG START
     *     if (CommandChunkDebug.chunkDebugEnabled) {
     *         CommandChunkDebug.onChunkPopulated(x, z, null);
     *     }
     *     // CHUNK DEBUG END
     *
     *     BlockFalling.fallInstantly = true;
     *     ...
     */
    public static void onChunkPopulated(int x, int z, String custom) {
        entries.add(new ChunkDebugEntry(x, z, getCurrentTick(), currentDimension, "POPULATED", collectMetadata(custom)));
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
     *         CommandChunkDebug.onChunkUnloadScheduled(x, z, null);
     *     }
     *     // CHUNK DEBUG END
     */
    public static void onChunkUnloadScheduled(int x, int z, String custom) {
        entries.add(new ChunkDebugEntry(x, z, getCurrentTick(), currentDimension, "UNLOAD_SCHEDULED", collectMetadata(custom)));
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
     *                  CommandChunkDebug.onChunkUnloaded(chunk.xPosition, chunk.zPosition, null);
     *              }
     *              // CHUNK DEBUG END
     *    ...
     */
    public static void onChunkUnloaded(int x, int z, String custom) {
        entries.add(new ChunkDebugEntry(x, z, getCurrentTick(), currentDimension, "UNLOADED", collectMetadata(custom)));
    }
}