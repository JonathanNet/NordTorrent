package com.nordtorrent.tv

import android.content.Context
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import java.io.IOException
import java.net.HttpURLConnection
import java.net.SocketTimeoutException
import java.net.URI
import java.net.URL
import java.net.UnknownHostException
import javax.net.ssl.SSLException

class ApiException(
    val statusCode: Int,
    message: String,
) : Exception(message)

class NetworkException(message: String, cause: Throwable? = null) : Exception(message, cause)

class ApiClient(
    baseUrl: String,
    private val token: String? = null,
    private val context: Context? = null,
) {
    val baseUrl: String = normalizeBaseUrl(baseUrl)

    suspend fun status(): Status = withContext(Dispatchers.IO) {
        val body = request("GET", "/api/v1/status", null, includeAuth = false)
        Status(
            setupRequired = body.optBoolean("setup_required", false),
            version = body.optString("version", ""),
            apiVersion = body.optInt("api_version", 1),
        )
    }

    suspend fun login(username: String, password: String): AuthResult = withContext(Dispatchers.IO) {
        val payload = JSONObject()
            .put("username", username)
            .put("password", password)
        val body = request("POST", "/api/v1/auth/login", payload, includeAuth = false)
        AuthResult(
            user = parseUser(body.getJSONObject("user")),
            token = body.getString("access_token"),
        )
    }

    suspend fun me(): User = withContext(Dispatchers.IO) {
        parseUser(request("GET", "/api/v1/auth/me", null))
    }

    suspend fun library(): List<LibraryItem> = withContext(Dispatchers.IO) {
        val body = request("GET", "/api/v1/library", null)
        val items = body.optJSONArray("items") ?: JSONArray()
        buildList {
            for (index in 0 until items.length()) {
                add(parseLibraryItem(items.getJSONObject(index)))
            }
        }
    }

    suspend fun torrents(): TorrentList = withContext(Dispatchers.IO) {
        val body = request("GET", "/api/v1/torrents", null)
        val items = body.optJSONArray("torrents") ?: JSONArray()
        val torrents = buildList {
            for (index in 0 until items.length()) {
                add(parseTorrent(items.getJSONObject(index)))
            }
        }
        TorrentList(torrents = torrents, totals = parseTotals(body.optJSONObject("totals")))
    }

    suspend fun pauseTorrent(id: Int) {
        withContext(Dispatchers.IO) {
            request("POST", "/api/v1/torrents/$id/pause", JSONObject())
        }
    }

    suspend fun resumeTorrent(id: Int) {
        withContext(Dispatchers.IO) {
            request("POST", "/api/v1/torrents/$id/resume", JSONObject())
        }
    }

    suspend fun logout() {
        withContext(Dispatchers.IO) {
            runCatching { request("POST", "/api/v1/auth/logout", JSONObject()) }
        }
    }

    fun streamUrl(path: String): String {
        if (path.startsWith("http://") || path.startsWith("https://")) return path
        return baseUrl + if (path.startsWith("/")) path else "/$path"
    }

    private fun request(
        method: String,
        path: String,
        body: JSONObject? = null,
        includeAuth: Boolean = true,
    ): JSONObject {
        val connection = try {
            URL(baseUrl + path).openConnection() as HttpURLConnection
        } catch (error: Exception) {
            throw NetworkException(
                problem(R.string.bad_url, "The server address is invalid."),
                error,
            )
        }
        try {
            connection.requestMethod = method
            connection.connectTimeout = 12_000
            connection.readTimeout = 25_000
            connection.setRequestProperty("Accept", "application/json")
            connection.setRequestProperty("X-Requested-With", "NordTorrent")
            if (includeAuth && !token.isNullOrBlank()) {
                connection.setRequestProperty("Authorization", "Bearer $token")
            }
            if (body != null) {
                connection.doOutput = true
                connection.setRequestProperty("Content-Type", "application/json; charset=utf-8")
                connection.outputStream.use { stream ->
                    stream.write(body.toString().toByteArray(Charsets.UTF_8))
                }
            }

            val status = connection.responseCode
            val stream = if (status in 200..299) connection.inputStream else connection.errorStream
            val text = stream?.bufferedReader(Charsets.UTF_8)?.use { it.readText() }.orEmpty()
            val parsed = if (text.isBlank()) JSONObject() else runCatching { JSONObject(text) }.getOrElse {
                if (status in 200..299) throw NetworkException(
                    problem(R.string.error_invalid_response, "The server returned an invalid response."),
                )
                JSONObject()
            }

            if (status !in 200..299) {
                val message = parsed.optJSONObject("error")?.optString("message")
                    ?.takeIf { it.isNotBlank() }
                    ?: defaultMessageFor(status)
                throw ApiException(status, message)
            }
            return parsed
        } catch (error: ApiException) {
            throw error
        } catch (error: SSLException) {
            Log.w(TAG, "TLS failed for $method $path", error)
            throw NetworkException(
                problem(
                    R.string.error_tls,
                    "The server certificate could not be validated. Use HTTPS with a valid certificate.",
                ),
                error,
            )
        } catch (error: UnknownHostException) {
            Log.w(TAG, "DNS failed for $method $path", error)
            throw NetworkException(
                problem(R.string.error_dns, "The server address could not be found."),
                error,
            )
        } catch (error: SocketTimeoutException) {
            Log.w(TAG, "Timeout for $method $path", error)
            throw NetworkException(
                problem(R.string.error_timeout, "The server did not respond in time."),
                error,
            )
        } catch (error: IOException) {
            Log.w(TAG, "I/O failed for $method $path", error)
            throw NetworkException(
                problem(R.string.connection_error, "Could not connect to the server."),
                error,
            )
        } finally {
            connection.disconnect()
        }
    }

    private fun defaultMessageFor(status: Int): String = when (status) {
        401 -> problem(R.string.error_unauthorized, "Wrong username or password.")
        403 -> problem(R.string.error_forbidden, "Your account is not allowed to do that.")
        404 -> problem(R.string.error_not_found, "The server could not find what was requested.")
        429 -> problem(R.string.error_rate_limited, "Too many attempts. Try again in a moment.")
        else -> context?.getString(R.string.error_server, status)
            ?: "The server responded with error $status."
    }

    private fun problem(resId: Int, fallback: String): String =
        context?.getString(resId) ?: fallback

    companion object {
        private const val TAG = "NordTorrentApi"

        fun normalizeBaseUrl(raw: String): String {
            var value = raw.trim()
            if (value.isEmpty()) throw IllegalArgumentException("The server address is missing.")
            if (!value.startsWith("http://", ignoreCase = true) &&
                !value.startsWith("https://", ignoreCase = true)
            ) {
                value = "http://$value"
            }
            val uri = try {
                URI(value)
            } catch (error: Exception) {
                throw IllegalArgumentException("The server address is invalid.")
            }
            if (uri.scheme.isNullOrBlank() || uri.host.isNullOrBlank()) {
                throw IllegalArgumentException("The server address is invalid.")
            }
            val defaultPort = if (uri.scheme.equals("https", true)) 443 else 80
            val port = if (uri.port == -1) defaultPort else uri.port
            val path = uri.path.orEmpty().trimEnd('/')
            return "${uri.scheme}://${uri.host}:$port$path"
        }

        private fun parseUser(json: JSONObject): User = User(
            username = json.optString("username"),
            displayName = json.optString("display_name").ifBlank { json.optString("username") },
            role = json.optString("role", "viewer"),
            createdAt = json.optString("created_at"),
        )

        private fun parseLibraryItem(json: JSONObject): LibraryItem = LibraryItem(
            torrentId = json.optInt("torrent_id"),
            fileId = json.optInt("file_id"),
            infoHash = json.optString("info_hash"),
            title = json.optString("title").ifBlank { json.optString("file_name") },
            fileName = json.optString("file_name"),
            torrentName = json.optString("torrent_name"),
            size = json.optLong("size"),
            downloadedBytes = json.optLong("downloaded_bytes"),
            progress = json.optDouble("progress", 0.0),
            kind = json.optString("kind", "video"),
            mime = json.optString("mime"),
            streamUrl = json.optString("stream_url"),
            torrentState = json.optString("torrent_state"),
        )

        private fun parseTorrent(json: JSONObject): TorrentSummary = TorrentSummary(
            id = json.optInt("id"),
            infoHash = json.optString("info_hash"),
            name = json.optString("name"),
            state = json.optString("state", "unknown"),
            progress = json.optDouble("progress", 0.0),
            progressBytes = json.optLong("progress_bytes"),
            totalBytes = json.optLong("total_bytes"),
            uploadedBytes = json.optLong("uploaded_bytes"),
            downloadSpeedBps = json.optDouble("download_speed_bps", 0.0),
            uploadSpeedBps = json.optDouble("upload_speed_bps", 0.0),
            etaSeconds = if (json.isNull("eta_seconds")) null else json.optLong("eta_seconds"),
            ratio = json.optDouble("ratio", 0.0),
            finished = json.optBoolean("finished"),
            error = if (json.isNull("error")) null else json.optString("error").takeIf { it.isNotBlank() },
            mediaFiles = json.optInt("media_files"),
            totalFiles = json.optInt("total_files"),
        )

        private fun parseTotals(json: JSONObject?): Totals {
            if (json == null) return Totals()
            return Totals(
                torrentCount = json.optInt("torrent_count"),
                active = json.optInt("active"),
                paused = json.optInt("paused"),
                completed = json.optInt("completed"),
                downloadSpeedBps = json.optDouble("download_speed_bps", 0.0),
                uploadSpeedBps = json.optDouble("upload_speed_bps", 0.0),
                downloadedBytes = json.optLong("downloaded_bytes"),
                uploadedBytes = json.optLong("uploaded_bytes"),
            )
        }
    }
}
