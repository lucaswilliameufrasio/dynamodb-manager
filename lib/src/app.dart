import 'package:flutter/material.dart';
import 'controllers/workspace_controller.dart';
import 'screens/profile_selection_screen.dart';

class DynamoDbClientApp extends StatefulWidget {
  const DynamoDbClientApp({super.key});

  @override
  State<DynamoDbClientApp> createState() => _DynamoDbClientAppState();
}

class _DynamoDbClientAppState extends State<DynamoDbClientApp> {
  late final WorkspaceController _controller;

  @override
  void initState() {
    super.initState();
    _controller = WorkspaceController();
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'DynamoDB Manager',
      theme: ThemeData.dark().copyWith(
        scaffoldBackgroundColor: Colors.black,
        colorScheme: ColorScheme.dark(
          primary: Colors.blueAccent,
          secondary: Colors.blueAccent,
          surface: Colors.grey.shade900,
        ),
      ),
      debugShowCheckedModeBanner: false,
      home: ProfileSelectionScreen(controller: _controller),
    );
  }
}
