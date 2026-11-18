import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../../providers/player_provider.dart';
import '../widgets/track_tile.dart';

class LibraryScreen extends StatefulWidget {
  const LibraryScreen({super.key});

  @override
  State<LibraryScreen> createState() => _LibraryScreenState();
}

class _LibraryScreenState extends State<LibraryScreen> {
  int _subTab = 0; // 0: Favoritos, 1: Playlists, 2: Offline, 3: Historial, 4: Cola
  String? _selectedPlaylist;

  @override
  Widget build(BuildContext context) {
    final provider = context.watch<PlayerProvider>();
    final accent = provider.accentColor;

    final tabs = [
      ('💚 Me Gusta (${provider.library.favorites.length})', 0),
      ('📂 Playlists (${provider.library.playlists.length})', 1),
      ('💾 Offline (${provider.library.downloads.length})', 2),
      ('🕒 Historial', 3),
      ('📑 Cola (${provider.queue.length})', 4),
    ];

    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 14, 16, 8),
          child: Row(
            children: [
              ClipRRect(
                borderRadius: BorderRadius.circular(8),
                child: Image.asset('assets/logo.jpg',
                    width: 34, height: 34, fit: BoxFit.cover),
              ),
              const SizedBox(width: 10),
              const Text(
                'Tu Biblioteca',
                style: TextStyle(
                    color: Colors.white,
                    fontSize: 22,
                    fontWeight: FontWeight.bold),
              ),
            ],
          ),
        ),
        SizedBox(
          height: 44,
          child: ListView.separated(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            scrollDirection: Axis.horizontal,
            itemCount: tabs.length,
            separatorBuilder: (_, _) => const SizedBox(width: 8),
            itemBuilder: (_, i) {
              final (label, idx) = tabs[i];
              final active = _subTab == idx;
              return ChoiceChip(
                label: Text(
                  label,
                  style: TextStyle(
                    color: active ? Colors.black : Colors.white,
                    fontWeight: FontWeight.bold,
                    fontSize: 12,
                  ),
                ),
                selected: active,
                selectedColor: accent,
                backgroundColor: const Color(0xFF242424),
                side: BorderSide.none,
                onSelected: (_) {
                  setState(() => _subTab = idx);
                  if (idx == 2) {
                    provider.library.scanLocalDownloads();
                  }
                },
              );
            },
          ),
        ),
        const SizedBox(height: 8),
        Expanded(child: _buildSubTab(provider, accent)),
      ],
    );
  }

  Widget _buildSubTab(PlayerProvider provider, Color accent) {
    switch (_subTab) {
      case 0:
        final favs = provider.library.favorites;
        if (favs.isEmpty) {
          return const Center(
            child: Text('Aún no tienes favoritas. Toca el corazón ❤️',
                style: TextStyle(color: Colors.grey)),
          );
        }
        return Column(
          children: [
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
              child: Row(
                children: [
                  ElevatedButton.icon(
                    style: ElevatedButton.styleFrom(
                        backgroundColor: accent, foregroundColor: Colors.black),
                    onPressed: () => provider.playQueue(favs),
                    icon: const Icon(Icons.play_arrow),
                    label: const Text('Reproducir Todo'),
                  ),
                ],
              ),
            ),
            Expanded(
              child: ListView.builder(
                padding: const EdgeInsets.only(bottom: 110),
                itemCount: favs.length,
                itemBuilder: (_, i) => TrackTile(track: favs[i], index: i),
              ),
            ),
          ],
        );

      case 1:
        return _buildPlaylistsTab(provider, accent);

      case 2:
        final downs = provider.library.downloads;
        if (downs.isEmpty) {
          return const Center(
            child: Text('No hay descargas offline. Usa ⋮ -> Descargar Offline',
                style: TextStyle(color: Colors.grey)),
          );
        }
        return Column(
          children: [
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
              child: Row(
                children: [
                  ElevatedButton.icon(
                    style: ElevatedButton.styleFrom(
                        backgroundColor: accent, foregroundColor: Colors.black),
                    onPressed: () => provider.playQueue(downs),
                    icon: const Icon(Icons.offline_bolt),
                    label: const Text('Reproducir Offline'),
                  ),
                  const Spacer(),
                  IconButton(
                    icon: const Icon(Icons.refresh, color: Colors.white70),
                    onPressed: () async {
                      await provider.library.scanLocalDownloads();
                      setState(() {});
                    },
                  ),
                ],
              ),
            ),
            Expanded(
              child: ListView.builder(
                padding: const EdgeInsets.only(bottom: 110),
                itemCount: downs.length,
                itemBuilder: (_, i) => TrackTile(track: downs[i], index: i),
              ),
            ),
          ],
        );

      case 3:
        final hist = provider.library.history;
        if (hist.isEmpty) {
          return const Center(
              child: Text('Historial vacío', style: TextStyle(color: Colors.grey)));
        }
        return ListView.builder(
          padding: const EdgeInsets.only(bottom: 110),
          itemCount: hist.length,
          itemBuilder: (_, i) => TrackTile(track: hist[i], index: i),
        );

      default:
        final q = provider.queue;
        if (q.isEmpty) {
          return const Center(
              child: Text('Cola vacía', style: TextStyle(color: Colors.grey)));
        }
        return Column(
          children: [
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.end,
                children: [
                  TextButton.icon(
                    onPressed: () {
                      provider.queue.clear();
                      provider.currentIndex = null;
                      provider.audioHandler.stop();
                      setState(() {});
                    },
                    icon: const Icon(Icons.clear_all, color: Colors.redAccent),
                    label: const Text('Vaciar Cola',
                        style: TextStyle(color: Colors.redAccent)),
                  ),
                ],
              ),
            ),
            Expanded(
              child: ListView.builder(
                padding: const EdgeInsets.only(bottom: 110),
                itemCount: q.length,
                itemBuilder: (_, i) => TrackTile(
                  track: q[i],
                  index: i,
                  onTap: () => provider.playQueueIndex(i),
                ),
              ),
            ),
          ],
        );
    }
  }

  Widget _buildPlaylistsTab(PlayerProvider provider, Color accent) {
    final names = provider.library.playlists.keys.toList()..sort();
    if (_selectedPlaylist != null && names.contains(_selectedPlaylist)) {
      final list = provider.library.playlists[_selectedPlaylist] ?? [];
      return Column(
        children: [
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12),
            child: Row(
              children: [
                IconButton(
                  icon: const Icon(Icons.arrow_back, color: Colors.white),
                  onPressed: () => setState(() => _selectedPlaylist = null),
                ),
                Expanded(
                  child: Text(
                    '📂 $_selectedPlaylist (${list.length})',
                    style: const TextStyle(
                        color: Colors.white,
                        fontWeight: FontWeight.bold,
                        fontSize: 16),
                  ),
                ),
                if (list.isNotEmpty)
                  ElevatedButton(
                    style: ElevatedButton.styleFrom(
                        backgroundColor: accent, foregroundColor: Colors.black),
                    onPressed: () => provider.playQueue(list),
                    child: const Text('▶ Todo'),
                  ),
                IconButton(
                  icon: const Icon(Icons.delete_outline, color: Colors.redAccent),
                  onPressed: () {
                    provider.library.deletePlaylist(_selectedPlaylist!);
                    setState(() => _selectedPlaylist = null);
                  },
                ),
              ],
            ),
          ),
          Expanded(
            child: list.isEmpty
                ? const Center(
                    child: Text('Playlist vacía',
                        style: TextStyle(color: Colors.grey)))
                : ListView.builder(
                    padding: const EdgeInsets.only(bottom: 110),
                    itemCount: list.length,
                    itemBuilder: (_, i) => TrackTile(
                      track: list[i],
                      index: i,
                      playlistContextName: _selectedPlaylist,
                    ),
                  ),
          ),
        ],
      );
    }

    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
          child: Row(
            children: [
              ElevatedButton.icon(
                style: ElevatedButton.styleFrom(
                    backgroundColor: accent, foregroundColor: Colors.black),
                onPressed: () => _showCreateDialog(provider, accent),
                icon: const Icon(Icons.add),
                label: const Text('Nueva Playlist'),
              ),
            ],
          ),
        ),
        Expanded(
          child: names.isEmpty
              ? const Center(
                  child: Text('Crea tu primera playlist personalizada',
                      style: TextStyle(color: Colors.grey)))
              : ListView.builder(
                  padding: const EdgeInsets.only(bottom: 110),
                  itemCount: names.length,
                  itemBuilder: (_, i) {
                    final name = names[i];
                    final count = provider.library.playlists[name]?.length ?? 0;
                    return ListTile(
                      leading: Container(
                        width: 46,
                        height: 46,
                        decoration: BoxDecoration(
                          color: const Color(0xFF262626),
                          borderRadius: BorderRadius.circular(8),
                        ),
                        child: Icon(Icons.folder_special, color: accent),
                      ),
                      title: Text(name,
                          style: const TextStyle(
                              color: Colors.white, fontWeight: FontWeight.bold)),
                      subtitle: Text('$count canciones',
                          style: TextStyle(color: Colors.grey.shade400)),
                      trailing: const Icon(Icons.chevron_right, color: Colors.grey),
                      onTap: () => setState(() => _selectedPlaylist = name),
                    );
                  },
                ),
        ),
      ],
    );
  }

  void _showCreateDialog(PlayerProvider provider, Color accent) {
    final ctrl = TextEditingController();
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        backgroundColor: const Color(0xFF1E1E1E),
        title: const Text('Nueva Playlist', style: TextStyle(color: Colors.white)),
        content: TextField(
          controller: ctrl,
          style: const TextStyle(color: Colors.white),
          decoration: const InputDecoration(
            hintText: 'Nombre de la playlist',
            hintStyle: TextStyle(color: Colors.grey),
          ),
        ),
        actions: [
          TextButton(
              onPressed: () => Navigator.pop(ctx),
              child: const Text('Cancelar')),
          ElevatedButton(
            style: ElevatedButton.styleFrom(
                backgroundColor: accent, foregroundColor: Colors.black),
            onPressed: () {
              final name = ctrl.text.trim();
              if (name.isNotEmpty) {
                provider.library.createPlaylist(name);
                setState(() => _selectedPlaylist = name);
                Navigator.pop(ctx);
              }
            },
            child: const Text('Crear'),
          ),
        ],
      ),
    );
  }
}
