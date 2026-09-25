import 'dart:convert';

class DynamoItem {
  final String id;
  final String jsonContent;
  final bool isEmpty;

  const DynamoItem({
    required this.id,
    required this.jsonContent,
    this.isEmpty = false,
  });

  factory DynamoItem.fromDynamoJson(String source) {
    final data = jsonDecode(source);
    var label = '(empty item)';
    if (data is Map && data.isNotEmpty) {
      final firstKey = data.keys.first;
      label = '$firstKey: ${data[firstKey]}';
    }
    const encoder = JsonEncoder.withIndent('  ');
    return DynamoItem(id: label, jsonContent: encoder.convert(data));
  }

  factory DynamoItem.empty() =>
      const DynamoItem(id: '', jsonContent: '', isEmpty: true);

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is DynamoItem &&
          runtimeType == other.runtimeType &&
          id == other.id &&
          jsonContent == other.jsonContent;

  @override
  int get hashCode => Object.hash(id, jsonContent);
}
