package com.myvidcomp.app

import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.provider.Settings
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel
import java.io.File

/**
 * Answers the three questions the interface cannot work out for itself on
 * Android: where the app's own programs were unpacked, whether it may read the
 * user's files, and which storage volumes exist.
 */
class MainActivity : FlutterActivity() {
    private val channelName = "myvidcomp/native"

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)

        MethodChannel(
            flutterEngine.dartExecutor.binaryMessenger,
            channelName,
        ).setMethodCallHandler { call, result ->
            when (call.method) {
                // Android only permits running a program from this folder, so
                // ffmpeg and ffprobe are shipped as libraries and live here.
                "nativeLibraryDir" -> result.success(applicationInfo.nativeLibraryDir)
                "ensureFileAccess" -> result.success(ensureFileAccess())
                "storageRoots" -> result.success(storageRoots())
                else -> result.notImplemented()
            }
        }
    }

    /**
     * Reports whether the app may work with ordinary files, opening the system
     * settings screen when it may not. This permission has no in-app dialog:
     * the user has to grant it in Settings, so the best the app can do is take
     * them there.
     */
    private fun ensureFileAccess(): Boolean {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.R) {
            return true
        }
        if (Environment.isExternalStorageManager()) {
            return true
        }

        runCatching {
            startActivity(
                Intent(
                    Settings.ACTION_MANAGE_APP_ALL_FILES_ACCESS_PERMISSION,
                    Uri.parse("package:$packageName"),
                ),
            )
        }.onFailure {
            // Some devices only offer the whole-system list.
            runCatching {
                startActivity(Intent(Settings.ACTION_MANAGE_ALL_FILES_ACCESS_PERMISSION))
            }
        }
        return false
    }

    /**
     * Lists the storage volumes a person would recognise, so the folder browser
     * can start somewhere useful rather than at the filesystem root.
     */
    private fun storageRoots(): List<String> {
        val roots = mutableListOf<String>()

        val primary = Environment.getExternalStorageDirectory()
        if (primary != null && primary.isDirectory) {
            roots.add(primary.absolutePath)
        }

        // Removable cards and drives appear as extra volumes; the app's own
        // per-volume folder sits four levels below the volume root.
        getExternalFilesDirs(null)
            .filterNotNull()
            .mapNotNull { volumeRoot(it) }
            .forEach { path -> if (!roots.contains(path)) roots.add(path) }

        return roots
    }

    /** Walks up from an app-private folder to the volume it sits on. */
    private fun volumeRoot(appFolder: File): String? {
        // .../<volume>/Android/data/<package>/files
        var candidate: File? = appFolder
        repeat(4) { candidate = candidate?.parentFile }
        val root = candidate ?: return null
        return if (root.isDirectory) root.absolutePath else null
    }
}
